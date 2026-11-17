use rustix::{
    io::Errno,
    net::{
        RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags, SendAncillaryBuffer, SendAncillaryMessage,
        SendFlags, recvmsg, sendmsg,
    },
};
use std::{
    collections::{HashMap, VecDeque, hash_map::Entry},
    io::{self, IoSlice, IoSliceMut},
    mem::{self, MaybeUninit},
    os::{
        fd::{AsFd, BorrowedFd, OwnedFd},
        unix::net::UnixStream,
    },
};

include!(concat!(env!("OUT_DIR"), "/protocols.rs"));

const MAX_FDS: usize = 28;
const MAX_QUEUED_FDS: usize = 256;
const MAX_MESSAGE: usize = 4096;
const MAX_PENDING_OUT: usize = 4 << 20;
const SERVER_ID_START: u32 = 0xff00_0000;
pub const DISPLAY: Obj = Obj { id: 1, version: 1 };

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Obj {
    pub id: u32,
    pub version: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fixed(pub i32);

impl From<f64> for Fixed {
    fn from(v: f64) -> Self {
        Fixed((v * 256.0).round() as i32)
    }
}

impl From<Fixed> for f64 {
    fn from(v: Fixed) -> Self {
        f64::from(v.0) / 256.0
    }
}

#[derive(Debug)]
pub struct NewId {
    pub interface: String,
    pub version: u32,
    pub id: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Cx {
    pub client: u64,
    pub obj: Obj,
    pub data: u32,
}

impl Cx {
    pub fn key(self) -> (u64, u32) {
        (self.client, self.obj.id)
    }

    pub fn surface(self) -> (u64, u32) {
        (self.client, self.data)
    }

    pub fn error(self, code: u32, message: impl Into<String>) -> Error {
        Error::new(self.obj.id, code, message)
    }

    pub fn error_if(self, failed: bool, code: u32, message: &str) -> Res {
        if failed { Err(self.error(code, message)) } else { Ok(()) }
    }
}

pub struct Error {
    pub object: u32,
    pub code: u32,
    pub message: String,
}

impl Error {
    pub fn new(object: u32, code: u32, message: impl Into<String>) -> Self {
        Self { object, code, message: message.into() }
    }

    fn malformed(object: u32, interface: &str, e: Malformed) -> Self {
        Self::new(object, wl_display::error::INVALID_METHOD, format!("{interface}: {e:?}"))
    }
}

pub type Res = Result<(), Error>;

pub trait Reply {
    fn reply(self) -> Res;
}

impl Reply for () {
    fn reply(self) -> Res {
        Ok(())
    }
}

impl Reply for Res {
    fn reply(self) -> Res {
        self
    }
}

pub trait Server {
    type Data: Default;

    fn connection(&mut self, client: u64) -> &mut Connection<Self::Data>;
}

#[derive(Debug)]
pub enum Malformed {
    Opcode,
    Truncated,
    NullObject,
    MissingFd,
    BadString,
    BadHeader,
}

pub struct Msg<'a> {
    pub id: u32,
    pub opcode: u16,
    pub args: Args<'a>,
}

pub struct Args<'a> {
    data: &'a [u8],
    fds: &'a mut VecDeque<OwnedFd>,
}

impl Args<'_> {
    pub fn uint(&mut self) -> Result<u32, Malformed> {
        let (word, rest) = self.data.split_first_chunk().ok_or(Malformed::Truncated)?;
        self.data = rest;
        Ok(u32::from_ne_bytes(*word))
    }

    pub fn int(&mut self) -> Result<i32, Malformed> {
        Ok(self.uint()? as i32)
    }

    pub fn fixed(&mut self) -> Result<Fixed, Malformed> {
        Ok(Fixed(self.int()?))
    }

    pub fn object_opt(&mut self) -> Result<Option<u32>, Malformed> {
        Ok(Some(self.uint()?).filter(|&id| id != 0))
    }

    pub fn object(&mut self) -> Result<u32, Malformed> {
        self.object_opt()?.ok_or(Malformed::NullObject)
    }

    fn bytes(&mut self) -> Result<&[u8], Malformed> {
        let len = self.uint()? as usize;
        let (bytes, rest) = self.data.split_at_checked(len.next_multiple_of(4)).ok_or(Malformed::Truncated)?;
        self.data = rest;
        Ok(&bytes[..len])
    }

    pub fn array(&mut self) -> Result<Vec<u8>, Malformed> {
        self.bytes().map(<[u8]>::to_vec)
    }

    pub fn string_opt(&mut self) -> Result<Option<String>, Malformed> {
        let bytes = self.bytes()?;
        let Some(text) = bytes.strip_suffix(&[0]) else {
            return if bytes.is_empty() { Ok(None) } else { Err(Malformed::BadString) };
        };
        std::str::from_utf8(text).map(|t| Some(t.to_owned())).map_err(|_| Malformed::BadString)
    }

    pub fn string(&mut self) -> Result<String, Malformed> {
        self.string_opt()?.ok_or(Malformed::NullObject)
    }

    pub fn new_id(&mut self) -> Result<NewId, Malformed> {
        Ok(NewId { interface: self.string()?, version: self.uint()?, id: self.object()? })
    }

    pub fn fd(&mut self) -> Result<OwnedFd, Malformed> {
        self.fds.pop_front().ok_or(Malformed::MissingFd)
    }
}

#[derive(Default)]
pub struct Input {
    buf: Vec<u8>,
    start: usize,
    end: usize,
    fds: VecDeque<OwnedFd>,
}

impl Input {
    pub fn recv(&mut self, socket: BorrowedFd) -> io::Result<usize> {
        if self.buf.is_empty() {
            self.buf.resize(2 * MAX_MESSAGE, 0);
        }
        self.buf.copy_within(self.start..self.end, 0);
        self.end -= self.start;
        self.start = 0;
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(MAX_FDS))];
        let mut control = RecvAncillaryBuffer::new(&mut space);
        let flags = RecvFlags::CMSG_CLOEXEC | RecvFlags::DONTWAIT;
        let n = loop {
            match recvmsg(socket, &mut [IoSliceMut::new(&mut self.buf[self.end..])], &mut control, flags) {
                Ok(r) if r.flags.contains(ReturnFlags::CTRUNC) => {
                    return Err(io::Error::other("descriptors truncated"));
                }
                Ok(r) => break r.bytes,
                Err(Errno::INTR) => continue,
                Err(e) => return Err(e.into()),
            }
        };
        for message in control.drain() {
            if let RecvAncillaryMessage::ScmRights(fds) = message {
                self.fds.extend(fds);
            }
        }
        if self.fds.len() > MAX_QUEUED_FDS {
            return Err(io::Error::other("too many queued descriptors"));
        }
        self.end += n;
        Ok(n)
    }

    pub fn message(&mut self) -> Result<Option<Msg<'_>>, Malformed> {
        let Some(header) = self.buf[self.start..self.end].first_chunk::<8>() else {
            return Ok(None);
        };
        let id = u32::from_ne_bytes(header[..4].try_into().unwrap());
        let word = u32::from_ne_bytes(header[4..].try_into().unwrap());
        let size = (word >> 16) as usize;
        if size < 8 || !size.is_multiple_of(4) || size > MAX_MESSAGE {
            return Err(Malformed::BadHeader);
        }
        if self.end - self.start < size {
            return Ok(None);
        }
        let data = &self.buf[self.start + 8..self.start + size];
        self.start += size;
        Ok(Some(Msg { id, opcode: word as u16, args: Args { data, fds: &mut self.fds } }))
    }
}

#[derive(Default)]
pub struct Out {
    bytes: Vec<u8>,
    fds: VecDeque<(usize, OwnedFd)>,
    message: usize,
}

impl Out {
    pub fn begin(&mut self, id: u32, opcode: u16) -> usize {
        let at = self.bytes.len();
        self.message = at;
        self.uint(id);
        self.uint(opcode as u32);
        at
    }

    pub fn end(&mut self, at: usize) {
        let word = &mut self.bytes[at + 4..at + 8];
        let opcode = u32::from_ne_bytes(word.try_into().unwrap());
        let size = (self.bytes.len() - at) as u32;
        self.bytes[at + 4..at + 8].copy_from_slice(&(size << 16 | opcode).to_ne_bytes());
    }

    pub fn uint(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_ne_bytes());
    }

    pub fn int(&mut self, v: i32) {
        self.uint(v as u32);
    }

    pub fn fixed(&mut self, v: Fixed) {
        self.int(v.0);
    }

    pub fn array(&mut self, v: &[u8]) {
        self.uint(v.len() as u32);
        self.bytes.extend_from_slice(v);
        self.bytes.resize(self.bytes.len().next_multiple_of(4), 0);
    }

    pub fn string(&mut self, v: Option<&str>) {
        match v {
            None => self.uint(0),
            Some(s) => {
                self.uint(s.len() as u32 + 1);
                self.bytes.extend_from_slice(s.as_bytes());
                self.bytes.push(0);
                self.bytes.resize(self.bytes.len().next_multiple_of(4), 0);
            }
        }
    }

    pub fn fd(&mut self, fd: BorrowedFd) {
        match fd.try_clone_to_owned() {
            Ok(fd) => self.fds.push_back((self.message, fd)),
            Err(e) => log::error!("cannot duplicate fd: {e}"),
        }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn flush(&mut self, socket: BorrowedFd) -> io::Result<bool> {
        while !self.bytes.is_empty() {
            let n_fds = self.fds.len().min(MAX_FDS);
            let limit = self.fds.get(MAX_FDS).map_or(self.bytes.len(), |f| f.0).max(1);
            match self.send(socket, n_fds, limit) {
                Ok(n) => {
                    self.fds.drain(..n_fds);
                    self.bytes.drain(..n);
                    self.fds.iter_mut().for_each(|f| f.0 = f.0.saturating_sub(n));
                    self.message = self.message.saturating_sub(n);
                }
                Err(Errno::AGAIN) => return Ok(false),
                Err(Errno::INTR) => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(true)
    }

    fn send(&self, socket: BorrowedFd, n_fds: usize, limit: usize) -> rustix::io::Result<usize> {
        let fds: Vec<_> = self.fds.iter().take(n_fds).map(|fd| fd.1.as_fd()).collect();
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(MAX_FDS))];
        let mut control = SendAncillaryBuffer::new(&mut space);
        if !fds.is_empty() {
            control.push(SendAncillaryMessage::ScmRights(&fds));
        }
        sendmsg(socket, &[IoSlice::new(&self.bytes[..limit])], &mut control, SendFlags::NOSIGNAL | SendFlags::DONTWAIT)
    }
}

pub struct Object<D> {
    pub interface: Interface,
    pub obj: Obj,
    pub data: u32,
    pub state: D,
}

pub struct Connection<D> {
    stream: UnixStream,
    input: Input,
    pub out: Out,
    objects: HashMap<u32, Object<D>>,
    next_server_id: u32,
    writing: bool,
}

impl<D: Default> Connection<D> {
    pub fn new(stream: UnixStream) -> Self {
        let display = Object { interface: Interface::WlDisplay, obj: DISPLAY, data: 0, state: D::default() };
        Self {
            stream,
            input: Input::default(),
            out: Out::default(),
            objects: HashMap::from([(DISPLAY.id, display)]),
            next_server_id: SERVER_ID_START,
            writing: false,
        }
    }

    pub fn add(&mut self, id: u32, interface: Interface, version: u32, data: u32) -> Result<Obj, Error> {
        let obj = Obj { id, version };
        match self.objects.entry(id) {
            Entry::Vacant(e) if id < SERVER_ID_START => e.insert(Object { interface, obj, data, state: D::default() }),
            _ => return Err(Error::new(DISPLAY.id, wl_display::error::INVALID_OBJECT, format!("invalid new id {id}"))),
        };
        Ok(obj)
    }

    pub fn add_server(&mut self, interface: Interface, version: u32, state: D) -> Obj {
        loop {
            let obj = Obj { id: self.next_server_id, version };
            self.next_server_id = obj.id.checked_add(1).unwrap_or(SERVER_ID_START);
            if let Entry::Vacant(e) = self.objects.entry(obj.id) {
                e.insert(Object { interface, obj, data: 0, state });
                return obj;
            }
        }
    }
}

impl<D> Connection<D> {
    pub fn get(&self, id: u32) -> Option<&Object<D>> {
        self.objects.get(&id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Object<D>> {
        self.objects.get_mut(&id)
    }

    pub fn get_of(&self, id: u32, interface: Interface) -> Option<&Object<D>> {
        self.get(id).filter(|o| o.interface == interface)
    }

    pub fn objects(&self) -> impl Iterator<Item = &Object<D>> {
        self.objects.values()
    }

    pub fn objects_mut(&mut self) -> impl Iterator<Item = &mut Object<D>> {
        self.objects.values_mut()
    }

    pub fn of(&self, interface: Interface) -> impl Iterator<Item = Obj> + '_ {
        self.objects().filter(move |o| o.interface == interface).map(|o| o.obj)
    }

    pub fn each(&mut self, interface: Interface, mut f: impl FnMut(&mut Out, Obj)) {
        for o in self.objects.values().filter(|o| o.interface == interface) {
            f(&mut self.out, o.obj);
        }
    }

    pub fn each_mut(&mut self, mut f: impl FnMut(&mut Out, &mut Object<D>)) {
        self.objects.values_mut().for_each(|o| f(&mut self.out, o));
    }

    pub fn set_data(&mut self, id: u32, data: u32) {
        if let Some(o) = self.objects.get_mut(&id) {
            o.data = data;
        }
    }

    pub fn take_all(&mut self) -> Vec<Object<D>> {
        self.objects.drain().map(|o| o.1).collect()
    }

    fn lookup(&self, client: u64, id: u32) -> Result<(Interface, Cx), Error> {
        let unknown = || Error::new(DISPLAY.id, wl_display::error::INVALID_OBJECT, format!("unknown object {id}"));
        let o = self.get(id).ok_or_else(unknown)?;
        Ok((o.interface, Cx { client, obj: o.obj, data: o.data }))
    }

    pub fn delete(&mut self, id: u32) -> Option<Object<D>> {
        let object = self.objects.remove(&id);
        if object.is_some() && id < SERVER_ID_START {
            wl_display::delete_id(&mut self.out, DISPLAY, id);
        }
        object
    }

    pub fn receive(&mut self) -> io::Result<Input> {
        let mut input = mem::take(&mut self.input);
        match input.recv(self.stream.as_fd()) {
            Ok(0) => Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(_) => Ok(input),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(input),
            Err(e) => Err(e),
        }
    }

    pub fn restore(&mut self, input: Input) {
        self.input = input;
    }

    pub fn post(&mut self, e: &Error) {
        wl_display::error(&mut self.out, DISPLAY, e.object, e.code, &e.message);
        let _ = self.out.flush(self.stream.as_fd());
    }

    pub fn flush(&mut self) -> io::Result<Option<bool>> {
        if self.out.is_empty() && !self.writing {
            return Ok(None);
        }
        let done = self.out.flush(self.stream.as_fd())?;
        if self.out.len() > MAX_PENDING_OUT {
            return Err(io::Error::other("client does not read"));
        }
        Ok((done == self.writing).then(|| {
            self.writing = !done;
            self.writing
        }))
    }
}

impl<D> AsFd for Connection<D> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.stream.as_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, os::unix::net::UnixStream};

    fn pair() -> (UnixStream, UnixStream) {
        let (client, server) = UnixStream::pair().unwrap();
        server.set_nonblocking(true).unwrap();
        (client, server)
    }

    fn send(socket: &UnixStream, bytes: &[u8], fds: &[BorrowedFd]) {
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(64))];
        let mut control = SendAncillaryBuffer::new(&mut space);
        if !fds.is_empty() {
            assert!(control.push(SendAncillaryMessage::ScmRights(fds)));
        }
        assert_eq!(sendmsg(socket, &[IoSlice::new(bytes)], &mut control, SendFlags::empty()).unwrap(), bytes.len());
    }

    fn message(id: u32, opcode: u16, args: &[u32]) -> Vec<u8> {
        let size = (8 + 4 * args.len() as u32) << 16 | u32::from(opcode);
        [id, size].iter().chain(args).flat_map(|w| w.to_ne_bytes()).collect()
    }

    fn parse(bytes: &[u8], read: impl FnOnce(&mut Args) -> Result<(), Malformed>) -> Result<(), Malformed> {
        let (client, server) = pair();
        send(&client, bytes, &[]);
        let mut input = Input::default();
        input.recv(server.as_fd()).unwrap();
        read(&mut input.message()?.expect("complete message").args)
    }

    #[test]
    fn reassembles_messages_split_across_reads_with_early_descriptors() {
        let (client, server) = pair();
        let file = File::open("/dev/null").unwrap();
        let mut out = Out::default();
        let at = out.begin(3, 1);
        out.string(Some("hello"));
        out.end(at);
        send(&client, &out.bytes[..6], &[file.as_fd()]);
        let mut input = Input::default();
        input.recv(server.as_fd()).unwrap();
        assert!(input.message().unwrap().is_none());
        send(&client, &out.bytes[6..], &[]);
        input.recv(server.as_fd()).unwrap();
        let mut msg = input.message().unwrap().unwrap();
        assert_eq!((msg.id, msg.opcode), (3, 1));
        assert_eq!(msg.args.string().unwrap(), "hello");
        assert!(msg.args.fd().is_ok());
        assert!(matches!(msg.args.fd(), Err(Malformed::MissingFd)));
        assert!(matches!(msg.args.uint(), Err(Malformed::Truncated)));
    }

    #[test]
    fn rejects_malformed_headers() {
        for size in [0, 4, 6, (MAX_MESSAGE + 4) as u32] {
            let bytes: Vec<u8> = [1, size << 16].iter().flat_map(|w: &u32| w.to_ne_bytes()).collect();
            assert!(matches!(parse(&bytes, |_| Ok(())), Err(Malformed::BadHeader)), "size {size}");
        }
    }

    #[test]
    fn rejects_arguments_past_the_message_end_and_bad_strings() {
        let truncated = parse(&message(1, 0, &[100]), |args| args.array().map(drop));
        assert!(matches!(truncated, Err(Malformed::Truncated)));
        let unterminated = parse(&message(1, 0, &[4, u32::from_ne_bytes(*b"abcd")]), |args| args.string().map(drop));
        assert!(matches!(unterminated, Err(Malformed::BadString)));
        let invalid = parse(&message(1, 0, &[2, u32::from_ne_bytes([0xff, 0, 0, 0])]), |args| args.string().map(drop));
        assert!(matches!(invalid, Err(Malformed::BadString)));
        let null = parse(&message(1, 0, &[0]), |args| args.object().map(drop));
        assert!(matches!(null, Err(Malformed::NullObject)));
    }

    #[test]
    fn flush_never_sends_descriptors_after_their_messages() {
        let (client, server) = pair();
        client.set_nonblocking(true).unwrap();
        let file = File::open("/dev/null").unwrap();
        let mut out = Out::default();
        for i in 0..2 * MAX_FDS as u32 + 5 {
            let at = out.begin(i + 1, 0);
            out.fd(file.as_fd());
            out.string(Some("x"));
            out.end(at);
        }
        assert!(out.flush(client.as_fd()).unwrap());
        let mut input = Input::default();
        let mut received = 0;
        while received < 2 * MAX_FDS as u32 + 5 {
            input.recv(server.as_fd()).unwrap();
            while let Some(mut msg) = input.message().unwrap() {
                received += 1;
                assert_eq!(msg.id, received);
                assert!(msg.args.fd().is_ok(), "message {received} lacks its descriptor");
                assert_eq!(msg.args.string().unwrap(), "x");
            }
        }
    }

    #[test]
    fn new_ids_must_be_unused_and_client_allocated() {
        let (stream, _peer) = UnixStream::pair().unwrap();
        let mut conn = Connection::<()>::new(stream);
        assert!(conn.add(DISPLAY.id, Interface::WlCallback, 1, 0).is_err());
        assert!(conn.add(2, Interface::WlCallback, 1, 0).is_ok());
        assert!(conn.add(2, Interface::WlCallback, 1, 0).is_err());
        assert!(conn.add(SERVER_ID_START, Interface::WlCallback, 1, 0).is_err());
        assert_eq!(conn.add_server(Interface::WlCallback, 1, ()).id, SERVER_ID_START);
        assert!(conn.get(3).is_none());
        conn.next_server_id = u32::MAX;
        assert_eq!(conn.add_server(Interface::WlCallback, 1, ()).id, u32::MAX);
        assert_eq!(conn.add_server(Interface::WlCallback, 1, ()).id, SERVER_ID_START + 1);
    }

    #[test]
    fn fixed_rounds() {
        assert_eq!(Fixed::from(0.999).0, 256);
        assert_eq!(Fixed::from(-0.999).0, -256);
    }

    #[test]
    fn refuses_descriptor_floods_and_truncation() {
        let (client, server) = pair();
        let file = File::open("/dev/null").unwrap();
        let mut input = Input::default();
        let mut flood = || {
            send(&client, &[0], &[file.as_fd(); MAX_FDS]);
            input.recv(server.as_fd()).is_ok()
        };
        let accepted = (0..MAX_QUEUED_FDS / MAX_FDS + 1).map(|_| flood()).collect::<Vec<_>>();
        assert_eq!(accepted.iter().filter(|&&ok| ok).count(), MAX_QUEUED_FDS / MAX_FDS);
        assert!(!accepted.last().unwrap());
        let (client, server) = pair();
        send(&client, &[0], &[file.as_fd(); MAX_FDS + 4]);
        assert!(Input::default().recv(server.as_fd()).is_err());
    }
}
