#![allow(unsafe_code)]

use crate::{Color, Rect, Source};
use ash::{Device, Entry, Instance, ext, khr, vk};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    ioctl::{Opcode, Setter, ioctl, opcode},
};
use std::{
    collections::HashMap,
    error::Error,
    ffi::{CStr, c_char},
    io::Cursor,
    os::fd::{AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd},
    ptr, slice,
    time::Duration,
};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

const TARGET_FORMAT: vk::Format = vk::Format::B8G8R8A8_UNORM;
const DMA_BUF: vk::ExternalMemoryHandleTypeFlags = vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT;
const SYNC_FD: vk::ExternalSemaphoreHandleTypeFlags = vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD;
const SLOTS: usize = 4;
const TIMEOUT: u64 = 1_000_000_000;
const DRM_FORMATS: [(u32, vk::Format, bool); 8] = [
    (0x3432_5241, vk::Format::B8G8R8A8_UNORM, false),
    (0x3432_5258, vk::Format::B8G8R8A8_UNORM, true),
    (0x3432_4241, vk::Format::R8G8B8A8_UNORM, false),
    (0x3432_4258, vk::Format::R8G8B8A8_UNORM, true),
    (0x3033_5241, vk::Format::A2R10G10B10_UNORM_PACK32, false),
    (0x3033_5258, vk::Format::A2R10G10B10_UNORM_PACK32, true),
    (0x3033_4241, vk::Format::A2B10G10R10_UNORM_PACK32, false),
    (0x3033_4258, vk::Format::A2B10G10R10_UNORM_PACK32, true),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TexId(u32);

pub struct Dmabuf {
    pub size: (i32, i32),
    pub format: u32,
    pub modifier: u64,
    pub planes: Vec<Plane>,
}

pub struct Plane {
    pub fd: OwnedFd,
    pub offset: u32,
    pub stride: u32,
}

pub enum Destination<'a> {
    Memory(&'a mut [u32]),
    Texture(TexId),
}

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub size: (i32, i32),
    pub transform: u32,
}

impl Frame {
    pub fn logical_size(&self) -> (i32, i32) {
        let (w, h) = self.size;
        if self.transform % 2 == 1 { (h, w) } else { (w, h) }
    }

    fn framebuffer_rect(self, r: Rect) -> Rect {
        let (fw, fh) = self.size;
        match self.transform & 3 {
            1 => Rect::new(r.y, fh - r.right(), r.h, r.w),
            2 => Rect::new(fw - r.right(), fh - r.bottom(), r.w, r.h),
            3 => Rect::new(fw - r.bottom(), r.x, r.h, r.w),
            _ => r,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Op {
    Fill(Rect, Color),
    Texture { tex: TexId, source: [f64; 4], transform: u32, dst: Rect, clip: Rect, alpha: f64 },
}

struct Texture {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
    size: (i32, i32),
    opaque: bool,
    foreign: bool,
}

struct Mapped {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    ptr: *mut u8,
    size: u64,
}

struct Target {
    texture: Texture,
    readback: Mapped,
}

struct Slot {
    cmd: vk::CommandBuffer,
    fence: vk::Fence,
    semaphore: vk::Semaphore,
    queries: vk::QueryPool,
    staging: Option<Mapped>,
    used: u64,
    seq: u64,
}

struct Upload {
    id: TexId,
    offset: u64,
    rect: Rect,
    discard: bool,
}

#[repr(C)]
struct Push {
    dst: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
    fill: u32,
    transform: u32,
    uv_transform: u32,
}

#[repr(C)]
struct ImportSyncFile {
    flags: u32,
    fd: i32,
}

const IMPORT_SYNC_FILE: Opcode = opcode::write::<ImportSyncFile>(b'b', 3);

pub struct Renderer {
    _entry: Entry,
    instance: Instance,
    device: Device,
    queue: vk::Queue,
    push_descriptor: khr::push_descriptor::Device,
    memory_fd: khr::external_memory_fd::Device,
    semaphore_fd: khr::external_semaphore_fd::Device,
    exportable: bool,
    memory_types: vk::PhysicalDeviceMemoryProperties,
    pool: vk::CommandPool,
    slots: Vec<Slot>,
    current: usize,
    submitted: u64,
    times: Vec<(u64, Duration)>,
    tick: f64,
    set_layout: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    sampler: vk::Sampler,
    family: u32,
    textures: HashMap<TexId, Texture>,
    deferred: Vec<(u64, Texture)>,
    next_id: u32,
    uploads: Vec<Upload>,
    target: Option<Target>,
    dummy: TexId,
    render_node: u64,
    formats: Vec<(u32, u64)>,
    target_formats: Vec<(u32, u64)>,
}

impl Renderer {
    pub fn new() -> Result<Self> {
        unsafe {
            let entry = Entry::load()?;
            let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);
            let instance = entry.create_instance(&vk::InstanceCreateInfo::default().application_info(&app), None)?;
            let (physical, render_node) = pick_device(&instance)?;
            let family = instance
                .get_physical_device_queue_family_properties(physical)
                .iter()
                .position(|f| f.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                .ok_or("no graphics queue")? as u32;
            let available = instance.enumerate_device_extension_properties(physical)?;
            let has = |name: &CStr| available.iter().any(|e| e.extension_name_as_c_str() == Ok(name));
            let priority = [khr::global_priority::NAME, ext::global_priority::NAME].into_iter().find(|&n| has(n));
            let mut extensions: Vec<*const c_char> = [
                khr::push_descriptor::NAME,
                khr::external_memory_fd::NAME,
                ext::external_memory_dma_buf::NAME,
                ext::image_drm_format_modifier::NAME,
                ext::queue_family_foreign::NAME,
                khr::external_semaphore_fd::NAME,
            ]
            .map(|name| name.as_ptr())
            .into();
            extensions.extend(priority.map(|name| name.as_ptr()));
            let device = create_device(&instance, physical, family, &extensions, priority.is_some())?;
            let queue = device.get_device_queue(family, 0);
            let mut properties = vk::ExternalSemaphoreProperties::default();
            let info = vk::PhysicalDeviceExternalSemaphoreInfo::default().handle_type(SYNC_FD);
            instance.get_physical_device_external_semaphore_properties(physical, &info, &mut properties);
            let exportable =
                properties.external_semaphore_features.contains(vk::ExternalSemaphoreFeatureFlags::EXPORTABLE);
            let pool_info = vk::CommandPoolCreateInfo::default()
                .queue_family_index(family)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
            let pool = device.create_command_pool(&pool_info, None)?;
            let buffers =
                vk::CommandBufferAllocateInfo::default().command_pool(pool).command_buffer_count(SLOTS as u32);
            let mut slots = vec![];
            for cmd in device.allocate_command_buffers(&buffers)? {
                let query_info = vk::QueryPoolCreateInfo::default().query_type(vk::QueryType::TIMESTAMP).query_count(2);
                slots.push(Slot {
                    cmd,
                    fence: device.create_fence(&vk::FenceCreateInfo::default(), None)?,
                    semaphore: create_semaphore(&device, exportable)?,
                    queries: device.create_query_pool(&query_info, None)?,
                    staging: None,
                    used: 0,
                    seq: 0,
                });
            }
            let tick = f64::from(instance.get_physical_device_properties(physical).limits.timestamp_period);
            let sampler_info = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE);
            let sampler = device.create_sampler(&sampler_info, None)?;
            let binding = vk::DescriptorSetLayoutBinding::default()
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT);
            let set_info = vk::DescriptorSetLayoutCreateInfo::default()
                .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR)
                .bindings(slice::from_ref(&binding));
            let set_layout = device.create_descriptor_set_layout(&set_info, None)?;
            let range = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .size(size_of::<Push>() as u32);
            let layout_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(slice::from_ref(&set_layout))
                .push_constant_ranges(slice::from_ref(&range));
            let layout = device.create_pipeline_layout(&layout_info, None)?;
            let pipeline = create_pipeline(&device, layout)?;
            let (formats, target_formats) = query_formats(&instance, physical);
            let mut renderer = Self {
                memory_types: instance.get_physical_device_memory_properties(physical),
                push_descriptor: khr::push_descriptor::Device::new(&instance, &device),
                memory_fd: khr::external_memory_fd::Device::new(&instance, &device),
                semaphore_fd: khr::external_semaphore_fd::Device::new(&instance, &device),
                exportable,
                formats,
                target_formats,
                _entry: entry,
                instance,
                device,
                queue,
                pool,
                slots,
                current: 0,
                submitted: 0,
                times: vec![],
                tick,
                set_layout,
                layout,
                pipeline,
                sampler,
                family,
                textures: HashMap::new(),
                deferred: vec![],
                next_id: 0,
                uploads: vec![],
                target: None,
                dummy: TexId(0),
                render_node,
            };
            let dummy = crate::Pixels { data: vec![0], width: 1, height: 1 };
            renderer.dummy = renderer.upload(None, &dummy)?;
            Ok(renderer)
        }
    }

    pub fn gpu_times(&mut self) -> Vec<(u64, Duration)> {
        std::mem::take(&mut self.times)
    }

    pub fn submitted(&self) -> u64 {
        self.submitted
    }

    pub fn completed(&self) -> u64 {
        self.slots.iter().filter(|s| s.seq != 0).map(|s| s.seq - 1).min().unwrap_or(u64::MAX)
    }

    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    pub fn render_node(&self) -> u64 {
        self.render_node
    }

    pub fn formats(&self) -> &[(u32, u64)] {
        &self.formats
    }

    pub fn target_formats(&self) -> &[(u32, u64)] {
        &self.target_formats
    }

    pub fn upload(&mut self, existing: Option<TexId>, src: &dyn Source) -> Result<TexId> {
        self.upload_damaged(existing, src, None)
    }

    pub fn upload_region(&mut self, existing: TexId, src: &dyn Source, region: Rect) -> Result<TexId> {
        self.upload_damaged(Some(existing), src, Some(region))
    }

    fn upload_damaged(&mut self, existing: Option<TexId>, src: &dyn Source, region: Option<Rect>) -> Result<TexId> {
        let (w, h) = src.size();
        if w <= 0 || h <= 0 {
            return Err("empty texture".into());
        }
        let reusable = |t: &Texture| t.size == (w, h) && t.opaque == src.opaque() && !t.foreign;
        let (id, region) = match existing.filter(|id| self.textures.get(id).is_some_and(reusable)) {
            Some(id) => (id, region),
            None => {
                existing.into_iter().for_each(|old| self.free(old));
                let usage = vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST;
                let info = image_info((w, h), TARGET_FORMAT, usage);
                let texture = unsafe { self.create_image(&info, src.opaque(), false, |i| self.allocate_image(i))? };
                (self.insert(texture), None)
            }
        };
        let full = Rect::new(0, 0, w, h);
        let rect = region.map_or(full, |r| r.intersect(full));
        if rect.is_empty() {
            return Ok(id);
        }
        let discard = rect == full;
        if discard {
            self.uploads.retain(|u| u.id != id);
        }
        let count = (rect.w * rect.h) as usize;
        let offset = self.reserve(count as u64 * 4)?;
        let staging = self.slots[self.current].staging.as_ref().ok_or("no staging buffer")?;
        unsafe {
            let pixels = slice::from_raw_parts_mut(staging.ptr.add(offset as usize).cast::<u32>(), count);
            for (row, y) in pixels.chunks_exact_mut(rect.w as usize).zip(rect.y..) {
                src.read_row(rect.x, y, row);
            }
        }
        self.uploads.push(Upload { id, offset, rect, discard });
        Ok(id)
    }

    fn reserve(&mut self, len: u64) -> Result<u64> {
        self.wait(self.current)?;
        let slot = &self.slots[self.current];
        let (used, capacity) = (slot.used, slot.staging.as_ref().map_or(0, |s| s.size));
        if used + len > capacity {
            let grown =
                self.mapped((used + len).max(capacity * 2).max(1 << 20), vk::BufferUsageFlags::TRANSFER_SRC, false)?;
            if let Some(old) = self.slots[self.current].staging.take() {
                unsafe { ptr::copy_nonoverlapping(old.ptr, grown.ptr, used as usize) };
                self.destroy_mapped(old);
            }
            self.slots[self.current].staging = Some(grown);
        }
        self.slots[self.current].used = used + len;
        Ok(used)
    }

    pub fn import(&mut self, dmabuf: &Dmabuf, target: bool) -> Result<TexId> {
        let &(_, format, opaque) = DRM_FORMATS.iter().find(|f| f.0 == dmabuf.format).ok_or("unsupported format")?;
        if target && format != TARGET_FORMAT {
            return Err("unsupported render target format".into());
        }
        let [plane] = dmabuf.planes.as_slice() else { return Err("only single-plane dmabufs are supported".into()) };
        let usage = if target { vk::ImageUsageFlags::COLOR_ATTACHMENT } else { vk::ImageUsageFlags::SAMPLED };
        let layout = vk::SubresourceLayout::default().offset(plane.offset.into()).row_pitch(plane.stride.into());
        let mut modifier = vk::ImageDrmFormatModifierExplicitCreateInfoEXT::default()
            .drm_format_modifier(dmabuf.modifier)
            .plane_layouts(slice::from_ref(&layout));
        let mut external = vk::ExternalMemoryImageCreateInfo::default().handle_types(DMA_BUF);
        let info = image_info(dmabuf.size, format, usage)
            .tiling(vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT)
            .push_next(&mut external)
            .push_next(&mut modifier);
        let texture =
            unsafe { self.create_image(&info, !target && opaque, true, |i| self.import_memory(i, &plane.fd))? };
        Ok(self.insert(texture))
    }

    pub fn free(&mut self, id: TexId) {
        let Some(t) = self.textures.remove(&id) else { return };
        self.uploads.retain(|u| u.id != id);
        match self.slots.iter().any(|s| s.seq != 0) {
            true => self.deferred.push((self.submitted, t)),
            false => self.destroy(&t),
        }
    }

    pub fn poll(&mut self) {
        for i in 0..SLOTS {
            let slot = &self.slots[i];
            if slot.seq != 0 && unsafe { self.device.get_fence_status(slot.fence) }.unwrap_or(false) {
                self.finish(i);
            }
        }
    }

    fn wait(&mut self, i: usize) -> Result<()> {
        if self.slots[i].seq == 0 {
            return Ok(());
        }
        match unsafe { self.device.wait_for_fences(&[self.slots[i].fence], true, TIMEOUT) } {
            Ok(()) => {
                self.finish(i);
                Ok(())
            }
            Err(vk::Result::TIMEOUT) => Err("the GPU did not finish within a second, treating it as lost".into()),
            Err(e) => Err(e.into()),
        }
    }

    fn finish(&mut self, i: usize) {
        let slot = &mut self.slots[i];
        let mut stamps = [0u64; 2];
        let flags = vk::QueryResultFlags::TYPE_64;
        if unsafe { self.device.get_query_pool_results(slot.queries, 0, &mut stamps, flags) }.is_ok() {
            let nanos = (stamps[1].saturating_sub(stamps[0]) as f64 * self.tick) as u64;
            self.times.push((slot.seq, Duration::from_nanos(nanos)));
            let excess = self.times.len().saturating_sub(16);
            self.times.drain(..excess);
        }
        let _ = unsafe { self.device.reset_fences(&[slot.fence]) };
        (slot.seq, slot.used) = (0, 0);
        let oldest = self.slots.iter().filter(|s| s.seq != 0).map(|s| s.seq).min().unwrap_or(u64::MAX);
        if self.deferred.iter().any(|d| d.0 < oldest) {
            let done: Vec<_> = self.deferred.extract_if(.., |d| d.0 < oldest).collect();
            done.iter().for_each(|(_, t)| self.destroy(t));
        }
    }

    pub fn render(
        &mut self,
        ops: &[Op],
        frame: Frame,
        destination: Destination,
        damage: Option<Rect>,
    ) -> Result<Option<OwnedFd>> {
        let (w, h) = frame.size;
        let partial = damage.filter(|_| matches!(destination, Destination::Texture(_)));
        if partial.is_some_and(|d| d.is_empty()) && self.uploads.is_empty() {
            return Ok(None);
        }
        let i = self.current;
        self.wait(i)?;
        let scanout = match destination {
            Destination::Texture(id) => {
                Some(self.textures.get(&id).map(|t| (t.image, t.view)).ok_or("unknown render target")?)
            }
            Destination::Memory(_) => {
                if self.target.as_ref().is_none_or(|t| t.texture.size != (w, h)) {
                    self.replace_target((w, h))?;
                }
                None
            }
        };
        let mut foreign: Vec<vk::Image> = ops
            .iter()
            .filter_map(|op| if let Op::Texture { tex, .. } = op { self.textures.get(tex) } else { None })
            .filter(|t| t.foreign)
            .map(|t| t.image)
            .collect();
        foreign.sort_unstable();
        foreign.dedup();
        let (cmd, queries) = (self.slots[i].cmd, self.slots[i].queries);
        let export = self.exportable && scanout.is_some();
        unsafe {
            let d = &self.device;
            d.reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;
            d.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default())?;
            d.cmd_reset_query_pool(cmd, queries, 0, 2);
            d.cmd_write_timestamp2(cmd, vk::PipelineStageFlags2::ALL_COMMANDS, queries, 0);
            self.record_uploads(cmd);
            let target =
                scanout.unwrap_or_else(|| self.target.as_ref().map(|t| (t.texture.image, t.texture.view)).unwrap());
            let barriers = |acquire: bool, target: vk::ImageMemoryBarrier2| {
                let mut barriers: Vec<_> =
                    foreign.iter().map(|&image| foreign_barrier(image, self.family, acquire)).collect();
                barriers.push(target);
                d.cmd_pipeline_barrier2(cmd, &vk::DependencyInfo::default().image_memory_barriers(&barriers));
            };
            let attachment = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
            barriers(
                true,
                match partial {
                    Some(_) => layout_barrier(target.0, vk::ImageLayout::GENERAL, attachment)
                        .src_queue_family_index(vk::QUEUE_FAMILY_FOREIGN_EXT)
                        .dst_queue_family_index(self.family),
                    None => layout_barrier(target.0, vk::ImageLayout::UNDEFINED, attachment),
                },
            );
            self.record_draws(cmd, ops, frame, target.1, partial);
            barriers(
                false,
                match scanout {
                    Some(_) => layout_barrier(target.0, attachment, vk::ImageLayout::GENERAL)
                        .src_queue_family_index(self.family)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_FOREIGN_EXT),
                    None => layout_barrier(target.0, attachment, vk::ImageLayout::TRANSFER_SRC_OPTIMAL),
                },
            );
            if let (None, Some(t)) = (scanout, &self.target) {
                let extent = vk::Extent3D { width: w as u32, height: h as u32, depth: 1 };
                let region = vk::BufferImageCopy::default().image_subresource(color_layers()).image_extent(extent);
                let layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                d.cmd_copy_image_to_buffer(cmd, t.texture.image, layout, t.readback.buffer, &[region]);
                let host = vk::BufferMemoryBarrier2::default()
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_TRANSFER)
                    .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::HOST)
                    .dst_access_mask(vk::AccessFlags2::HOST_READ)
                    .buffer(t.readback.buffer)
                    .size(vk::WHOLE_SIZE);
                d.cmd_pipeline_barrier2(cmd, &vk::DependencyInfo::default().buffer_memory_barriers(&[host]));
            }
            d.cmd_write_timestamp2(cmd, vk::PipelineStageFlags2::ALL_COMMANDS, queries, 1);
            d.end_command_buffer(cmd)?;
            let signal = [self.slots[i].semaphore];
            let signal: &[vk::Semaphore] = if export { &signal } else { &[] };
            let submit = vk::SubmitInfo::default().command_buffers(slice::from_ref(&cmd)).signal_semaphores(signal);
            d.queue_submit(self.queue, &[submit], self.slots[i].fence)?;
        }
        self.uploads.clear();
        self.submitted += 1;
        self.slots[i].seq = self.submitted;
        self.current = (i + 1) % SLOTS;
        match destination {
            Destination::Memory(out) => {
                self.wait(i)?;
                if let Some(t) = &self.target {
                    let len = out.len().min((w * h) as usize);
                    out[..len].copy_from_slice(unsafe { slice::from_raw_parts(t.readback.ptr.cast::<u32>(), len) });
                }
                Ok(None)
            }
            Destination::Texture(_) if export => self.export(i),
            Destination::Texture(_) => self.wait(i).map(|()| None),
        }
    }

    fn export(&mut self, i: usize) -> Result<Option<OwnedFd>> {
        let info = vk::SemaphoreGetFdInfoKHR::default().semaphore(self.slots[i].semaphore).handle_type(SYNC_FD);
        match unsafe { self.semaphore_fd.get_semaphore_fd(&info) } {
            Ok(fd) if fd >= 0 => Ok(Some(unsafe { OwnedFd::from_raw_fd(fd) })),
            Ok(_) => Ok(None),
            Err(e) => {
                log::warn!("cannot export the render fence, waiting for the GPU: {e}");
                self.wait(i)?;
                unsafe {
                    self.device.destroy_semaphore(self.slots[i].semaphore, None);
                    self.slots[i].semaphore = create_semaphore(&self.device, self.exportable)?;
                }
                Ok(None)
            }
        }
    }

    unsafe fn record_uploads(&self, cmd: vk::CommandBuffer) {
        let Some(staging) = self.slots[self.current].staging.as_ref().filter(|_| !self.uploads.is_empty()) else {
            return;
        };
        let d = &self.device;
        let mut first: Vec<(vk::Image, bool)> = vec![];
        for u in &self.uploads {
            let Some(t) = self.textures.get(&u.id) else { continue };
            if !first.iter().any(|f| f.0 == t.image) {
                first.push((t.image, u.discard));
            }
        }
        let (dst, read) = (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        let transition = |barriers: Vec<vk::ImageMemoryBarrier2>| unsafe {
            d.cmd_pipeline_barrier2(cmd, &vk::DependencyInfo::default().image_memory_barriers(&barriers))
        };
        transition(
            first
                .iter()
                .map(|&(image, discard)| {
                    layout_barrier(image, if discard { vk::ImageLayout::UNDEFINED } else { read }, dst)
                })
                .collect(),
        );
        for u in &self.uploads {
            let Some(t) = self.textures.get(&u.id) else { continue };
            let region = vk::BufferImageCopy::default()
                .buffer_offset(u.offset)
                .image_subresource(color_layers())
                .image_offset(vk::Offset3D { x: u.rect.x, y: u.rect.y, z: 0 })
                .image_extent(vk::Extent3D { width: u.rect.w as u32, height: u.rect.h as u32, depth: 1 });
            unsafe { d.cmd_copy_buffer_to_image(cmd, staging.buffer, t.image, dst, &[region]) };
        }
        transition(first.iter().map(|&(image, _)| layout_barrier(image, dst, read)).collect());
    }

    unsafe fn record_draws(
        &self,
        cmd: vk::CommandBuffer,
        ops: &[Op],
        frame: Frame,
        view: vk::ImageView,
        damage: Option<Rect>,
    ) {
        let (w, h) = frame.size;
        let (vw, vh) = frame.logical_size();
        let d = &self.device;
        let load = if damage.is_some() { vk::AttachmentLoadOp::LOAD } else { vk::AttachmentLoadOp::CLEAR };
        let attachment = vk::RenderingAttachmentInfo::default()
            .image_view(view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(load)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } });
        let extent = vk::Extent2D { width: w as u32, height: h as u32 };
        let rendering = vk::RenderingInfo::default()
            .render_area(extent.into())
            .layer_count(1)
            .color_attachments(slice::from_ref(&attachment));
        let bounds = damage.unwrap_or(Rect::new(0, 0, vw, vh));
        let ndc = |r: Rect| {
            let (fw, fh) = (vw as f32, vh as f32);
            [r.x as f32 / fw * 2.0 - 1.0, r.y as f32 / fh * 2.0 - 1.0, r.w as f32 / fw * 2.0, r.h as f32 / fh * 2.0]
        };
        unsafe {
            d.cmd_begin_rendering(cmd, &rendering);
            d.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            let viewport = vk::Viewport { width: w as f32, height: h as f32, max_depth: 1.0, ..Default::default() };
            d.cmd_set_viewport(cmd, 0, &[viewport]);
            for op in ops {
                let (tex, push, clip) = match *op {
                    Op::Fill(rect, color) => {
                        let push = Push {
                            dst: ndc(rect),
                            uv: [0.0; 4],
                            color: unpack(color),
                            fill: 1,
                            transform: frame.transform,
                            uv_transform: 0,
                        };
                        (self.dummy, push, rect)
                    }
                    Op::Texture { tex, source: [sx, sy, sw, sh], transform, dst, clip, alpha } => {
                        let Some(t) = self.textures.get(&tex) else { continue };
                        let (tw, th) = (t.size.0 as f64, t.size.1 as f64);
                        let uv = [sx / tw, sy / th, sw / tw, sh / th].map(|v| v as f32);
                        (
                            tex,
                            Push {
                                dst: ndc(dst),
                                uv,
                                color: [0.0, 0.0, 0.0, alpha as f32],
                                fill: 0,
                                transform: frame.transform,
                                uv_transform: transform,
                            },
                            clip.intersect(dst),
                        )
                    }
                };
                let clip = frame.framebuffer_rect(clip.intersect(bounds));
                let Some(view) = self.textures.get(&tex).map(|t| t.view).filter(|_| !clip.is_empty()) else { continue };
                let scissor = vk::Rect2D {
                    offset: vk::Offset2D { x: clip.x, y: clip.y },
                    extent: vk::Extent2D { width: clip.w as u32, height: clip.h as u32 },
                };
                d.cmd_set_scissor(cmd, 0, &[scissor]);
                let image = vk::DescriptorImageInfo::default()
                    .sampler(self.sampler)
                    .image_view(view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                let write = vk::WriteDescriptorSet::default()
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(&image));
                let point = vk::PipelineBindPoint::GRAPHICS;
                self.push_descriptor.cmd_push_descriptor_set(cmd, point, self.layout, 0, &[write]);
                let bytes = slice::from_raw_parts((&push as *const Push).cast::<u8>(), size_of::<Push>());
                let stages = vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT;
                d.cmd_push_constants(cmd, self.layout, stages, 0, bytes);
                d.cmd_draw(cmd, 4, 1, 0, 0);
            }
            d.cmd_end_rendering(cmd);
        }
    }

    fn insert(&mut self, texture: Texture) -> TexId {
        self.next_id += 1;
        let id = TexId(self.next_id);
        self.textures.insert(id, texture);
        id
    }

    unsafe fn create_image(
        &self,
        info: &vk::ImageCreateInfo,
        opaque: bool,
        foreign: bool,
        allocate: impl FnOnce(vk::Image) -> Result<vk::DeviceMemory>,
    ) -> Result<Texture> {
        let image = unsafe { self.device.create_image(info, None)? };
        let size = (info.extent.width as i32, info.extent.height as i32);
        let (memory, view) = (vk::DeviceMemory::null(), vk::ImageView::null());
        let mut t = Texture { image, memory, view, size, opaque, foreign };
        let setup = || -> Result<()> {
            t.memory = allocate(image)?;
            unsafe { self.device.bind_image_memory(image, t.memory, 0)? };
            t.view = unsafe { self.create_view(image, info.format, opaque)? };
            Ok(())
        };
        match setup() {
            Ok(()) => Ok(t),
            Err(e) => {
                self.destroy(&t);
                Err(e)
            }
        }
    }

    fn destroy(&self, t: &Texture) {
        unsafe {
            self.device.destroy_image_view(t.view, None);
            self.device.destroy_image(t.image, None);
            self.device.free_memory(t.memory, None);
        }
    }

    fn replace_target(&mut self, size: (i32, i32)) -> Result<()> {
        if let Some(old) = self.target.take() {
            self.destroy(&old.texture);
            self.destroy_mapped(old.readback);
        }
        let usage = vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC;
        let info = image_info(size, TARGET_FORMAT, usage);
        let texture = unsafe { self.create_image(&info, false, false, |i| self.allocate_image(i))? };
        match self.mapped((size.0 * size.1 * 4) as u64, vk::BufferUsageFlags::TRANSFER_DST, true) {
            Ok(readback) => self.target = Some(Target { texture, readback }),
            Err(e) => {
                self.destroy(&texture);
                return Err(e);
            }
        }
        Ok(())
    }

    fn mapped(&self, size: u64, usage: vk::BufferUsageFlags, cached: bool) -> Result<Mapped> {
        unsafe {
            let buffer = self.device.create_buffer(&vk::BufferCreateInfo::default().size(size).usage(usage), None)?;
            let requirements = self.device.get_buffer_memory_requirements(buffer);
            let host = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
            let preferred = if cached { host | vk::MemoryPropertyFlags::HOST_CACHED } else { host };
            let bits = requirements.memory_type_bits;
            let mut memory = vk::DeviceMemory::null();
            let mut setup = || -> Result<*mut u8> {
                let index = self.memory_type(bits, preferred).or_else(|| self.memory_type(bits, host));
                let info = vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(index.ok_or("no host visible memory")?);
                memory = self.device.allocate_memory(&info, None)?;
                self.device.bind_buffer_memory(buffer, memory, 0)?;
                Ok(self.device.map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())?.cast())
            };
            match setup() {
                Ok(ptr) => Ok(Mapped { buffer, memory, ptr, size }),
                Err(e) => {
                    self.destroy_mapped(Mapped { buffer, memory, ptr: ptr::null_mut(), size });
                    Err(e)
                }
            }
        }
    }

    fn destroy_mapped(&self, mapped: Mapped) {
        unsafe {
            self.device.destroy_buffer(mapped.buffer, None);
            self.device.free_memory(mapped.memory, None);
        }
    }

    unsafe fn allocate_image(&self, image: vk::Image) -> Result<vk::DeviceMemory> {
        unsafe {
            let requirements = self.device.get_image_memory_requirements(image);
            let flags = vk::MemoryPropertyFlags::DEVICE_LOCAL;
            let index = self.memory_type(requirements.memory_type_bits, flags).ok_or("no device memory")?;
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
            let info = vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(index)
                .push_next(&mut dedicated);
            Ok(self.device.allocate_memory(&info, None)?)
        }
    }

    unsafe fn import_memory(&self, image: vk::Image, fd: &OwnedFd) -> Result<vk::DeviceMemory> {
        unsafe {
            let mut properties = vk::MemoryFdPropertiesKHR::default();
            self.memory_fd.get_memory_fd_properties(DMA_BUF, fd.as_raw_fd(), &mut properties)?;
            let requirements = self.device.get_image_memory_requirements(image);
            let bits = requirements.memory_type_bits & properties.memory_type_bits;
            let index = self.memory_type(bits, vk::MemoryPropertyFlags::empty()).ok_or("no importable memory")?;
            let fd = fd.try_clone()?;
            let mut import = vk::ImportMemoryFdInfoKHR::default().handle_type(DMA_BUF).fd(fd.as_raw_fd());
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
            let info = vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(index)
                .push_next(&mut import)
                .push_next(&mut dedicated);
            let memory = self.device.allocate_memory(&info, None)?;
            let _ = fd.into_raw_fd();
            Ok(memory)
        }
    }

    unsafe fn create_view(&self, image: vk::Image, format: vk::Format, opaque: bool) -> Result<vk::ImageView> {
        let alpha = if opaque { vk::ComponentSwizzle::ONE } else { vk::ComponentSwizzle::IDENTITY };
        let info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .components(vk::ComponentMapping { a: alpha, ..Default::default() })
            .subresource_range(color_range());
        Ok(unsafe { self.device.create_image_view(&info, None)? })
    }

    fn memory_type(&self, bits: u32, flags: vk::MemoryPropertyFlags) -> Option<u32> {
        let types = &self.memory_types.memory_types[..self.memory_types.memory_type_count as usize];
        (0..types.len() as u32).find(|&i| bits & (1 << i) != 0 && types[i as usize].property_flags.contains(flags))
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
        }
        let deferred = std::mem::take(&mut self.deferred).into_iter().map(|d| d.1);
        for t in std::mem::take(&mut self.textures).into_values().chain(deferred) {
            self.destroy(&t);
        }
        if let Some(t) = self.target.take() {
            self.destroy(&t.texture);
            self.destroy_mapped(t.readback);
        }
        for slot in std::mem::take(&mut self.slots) {
            slot.staging.into_iter().for_each(|s| self.destroy_mapped(s));
            unsafe {
                self.device.destroy_fence(slot.fence, None);
                self.device.destroy_semaphore(slot.semaphore, None);
                self.device.destroy_query_pool(slot.queries, None);
            }
        }
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.layout, None);
            self.device.destroy_descriptor_set_layout(self.set_layout, None);
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_command_pool(self.pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

pub fn attach_fence(dmabuf: &Dmabuf, fence: BorrowedFd) {
    let Some(plane) = dmabuf.planes.first() else { return };
    let import = ImportSyncFile { flags: 2, fd: fence.as_raw_fd() };
    if unsafe { ioctl(&plane.fd, Setter::<IMPORT_SYNC_FILE, ImportSyncFile>::new(import)) }.is_ok() {
        return;
    }
    let mut fds = [PollFd::new(&fence, PollFlags::IN)];
    if !poll(&mut fds, Some(&Timespec { tv_sec: 1, tv_nsec: 0 })).is_ok_and(|n| n > 0) {
        log::warn!("the render fence did not signal within a second");
    }
}

unsafe fn create_semaphore(device: &Device, exportable: bool) -> Result<vk::Semaphore> {
    let mut export = vk::ExportSemaphoreCreateInfo::default().handle_types(SYNC_FD);
    let mut info = vk::SemaphoreCreateInfo::default();
    if exportable {
        info = info.push_next(&mut export);
    }
    Ok(unsafe { device.create_semaphore(&info, None)? })
}

unsafe fn create_device(
    instance: &Instance,
    physical: vk::PhysicalDevice,
    family: u32,
    extensions: &[*const c_char],
    prioritized: bool,
) -> Result<Device> {
    let levels = [Some(vk::QueueGlobalPriorityKHR::REALTIME), Some(vk::QueueGlobalPriorityKHR::HIGH), None];
    for level in levels.into_iter().filter(|l| prioritized || l.is_none()) {
        let mut features = vk::PhysicalDeviceVulkan13Features::default().dynamic_rendering(true).synchronization2(true);
        let priorities = [1.0];
        let mut global =
            vk::DeviceQueueGlobalPriorityCreateInfoKHR::default().global_priority(level.unwrap_or_default());
        let mut queue_info =
            vk::DeviceQueueCreateInfo::default().queue_family_index(family).queue_priorities(&priorities);
        if level.is_some() {
            queue_info = queue_info.push_next(&mut global);
        }
        let info = vk::DeviceCreateInfo::default()
            .queue_create_infos(slice::from_ref(&queue_info))
            .enabled_extension_names(extensions)
            .push_next(&mut features);
        match unsafe { instance.create_device(physical, &info, None) } {
            Ok(device) => {
                log::info!("gpu queue priority {:?}", level.unwrap_or(vk::QueueGlobalPriorityKHR::MEDIUM));
                return Ok(device);
            }
            Err(vk::Result::ERROR_NOT_PERMITTED_KHR | vk::Result::ERROR_INITIALIZATION_FAILED) if level.is_some() => {}
            Err(e) => return Err(e.into()),
        }
    }
    Err("cannot create a vulkan device".into())
}

unsafe fn pick_device(instance: &Instance) -> Result<(vk::PhysicalDevice, u64)> {
    for physical in unsafe { instance.enumerate_physical_devices()? } {
        let mut drm = vk::PhysicalDeviceDrmPropertiesEXT::default();
        let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut drm);
        unsafe { instance.get_physical_device_properties2(physical, &mut properties) };
        let device_type = properties.properties.device_type;
        if device_type != vk::PhysicalDeviceType::CPU && drm.has_render == vk::TRUE {
            return Ok((physical, rustix::fs::makedev(drm.render_major as u32, drm.render_minor as u32)));
        }
    }
    Err("no GPU with a render node".into())
}

type Formats = Vec<(u32, u64)>;

unsafe fn query_formats(instance: &Instance, physical: vk::PhysicalDevice) -> (Formats, Formats) {
    let (mut sampled, mut targets) = (vec![], vec![]);
    for &(fourcc, format, _) in &DRM_FORMATS {
        let count = {
            let mut list = vk::DrmFormatModifierPropertiesListEXT::default();
            let mut properties = vk::FormatProperties2::default().push_next(&mut list);
            unsafe { instance.get_physical_device_format_properties2(physical, format, &mut properties) };
            list.drm_format_modifier_count as usize
        };
        let mut modifiers = vec![vk::DrmFormatModifierPropertiesEXT::default(); count];
        {
            let mut list =
                vk::DrmFormatModifierPropertiesListEXT::default().drm_format_modifier_properties(&mut modifiers);
            let mut properties = vk::FormatProperties2::default().push_next(&mut list);
            unsafe { instance.get_physical_device_format_properties2(physical, format, &mut properties) };
        }
        let supports = |m: &vk::DrmFormatModifierPropertiesEXT, feature| {
            m.drm_format_modifier_plane_count == 1 && m.drm_format_modifier_tiling_features.contains(feature)
        };
        for m in &modifiers {
            if supports(m, vk::FormatFeatureFlags::SAMPLED_IMAGE) {
                sampled.push((fourcc, m.drm_format_modifier));
            }
            if format == TARGET_FORMAT && supports(m, vk::FormatFeatureFlags::COLOR_ATTACHMENT) {
                targets.push((fourcc, m.drm_format_modifier));
            }
        }
    }
    (sampled, targets)
}

unsafe fn create_pipeline(device: &Device, layout: vk::PipelineLayout) -> Result<vk::Pipeline> {
    let module = |code: &[u8]| -> Result<vk::ShaderModule> {
        let words = ash::util::read_spv(&mut Cursor::new(code))?;
        Ok(unsafe { device.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&words), None)? })
    };
    let vertex = module(include_bytes!(concat!(env!("OUT_DIR"), "/quad.vert.spv")))?;
    let fragment = module(include_bytes!(concat!(env!("OUT_DIR"), "/quad.frag.spv")))?;
    let stages = [(vk::ShaderStageFlags::VERTEX, vertex), (vk::ShaderStageFlags::FRAGMENT, fragment)]
        .map(|(stage, module)| vk::PipelineShaderStageCreateInfo::default().stage(stage).module(module).name(c"main"));
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
    let assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_STRIP);
    let viewport = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
    let raster = vk::PipelineRasterizationStateCreateInfo::default().line_width(1.0);
    let multisample =
        vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let blend = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::ONE)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA);
    let blending = vk::PipelineColorBlendStateCreateInfo::default().attachments(slice::from_ref(&blend));
    let dynamic = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic);
    let mut rendering =
        vk::PipelineRenderingCreateInfo::default().color_attachment_formats(slice::from_ref(&TARGET_FORMAT));
    let info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&assembly)
        .viewport_state(&viewport)
        .rasterization_state(&raster)
        .multisample_state(&multisample)
        .color_blend_state(&blending)
        .dynamic_state(&dynamic)
        .layout(layout)
        .push_next(&mut rendering);
    let pipeline = unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None) };
    unsafe {
        device.destroy_shader_module(vertex, None);
        device.destroy_shader_module(fragment, None);
    }
    Ok(pipeline.map_err(|(_, e)| e)?[0])
}

fn image_info<'a>(size: (i32, i32), format: vk::Format, usage: vk::ImageUsageFlags) -> vk::ImageCreateInfo<'a> {
    vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D { width: size.0 as u32, height: size.1 as u32, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .usage(usage)
        .initial_layout(vk::ImageLayout::UNDEFINED)
}

fn color_range() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange::default().aspect_mask(vk::ImageAspectFlags::COLOR).level_count(1).layer_count(1)
}

fn color_layers() -> vk::ImageSubresourceLayers {
    vk::ImageSubresourceLayers::default().aspect_mask(vk::ImageAspectFlags::COLOR).layer_count(1)
}

fn layout_barrier<'a>(image: vk::Image, old: vk::ImageLayout, new: vk::ImageLayout) -> vk::ImageMemoryBarrier2<'a> {
    vk::ImageMemoryBarrier2::default()
        .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
        .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .dst_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
        .old_layout(old)
        .new_layout(new)
        .image(image)
        .subresource_range(color_range())
}

fn foreign_barrier<'a>(image: vk::Image, family: u32, acquire: bool) -> vk::ImageMemoryBarrier2<'a> {
    let (general, read) = (vk::ImageLayout::GENERAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    let foreign = vk::QUEUE_FAMILY_FOREIGN_EXT;
    match acquire {
        true => layout_barrier(image, general, read).src_queue_family_index(foreign).dst_queue_family_index(family),
        false => layout_barrier(image, read, general).src_queue_family_index(family).dst_queue_family_index(foreign),
    }
}

fn unpack(color: Color) -> [f32; 4] {
    let channel = |shift: u32| ((color >> shift) & 0xff) as f32 / 255.0;
    let alpha = channel(24);
    [channel(16) * alpha, channel(8) * alpha, channel(0) * alpha, alpha]
}
