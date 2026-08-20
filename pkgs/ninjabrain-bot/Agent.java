package ninjabrainbot.ipc;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.lang.instrument.Instrumentation;
import java.net.StandardProtocolFamily;
import java.net.UnixDomainSocketAddress;
import java.nio.channels.Channels;
import java.nio.channels.ClosedChannelException;
import java.nio.channels.ServerSocketChannel;
import java.nio.channels.SocketChannel;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import com.github.kwhat.jnativehook.GlobalScreen;
import com.github.kwhat.jnativehook.NativeHookException;
import ninjabrainbot.io.ClipboardBridge;
import ninjabrainbot.io.ClipboardReader;
import ninjabrainbot.io.preferences.HotkeyBridge;
import ninjabrainbot.io.preferences.HotkeyPreference;

/**
 * Lets the bot be driven from outside, and gives it back the input a Wayland
 * session takes away.
 *
 * <p>Loaded as a java agent, this listens on a unix socket and triggers the
 * bot's actions straight off it, below the level a hotkey would have come in
 * at. On any session that is worth having -- anything that can bind a key --
 * that is all it does, and the bot behaves exactly as it always has.
 *
 * <p>Under Wayland it also has to stand in for what the bot can no longer do
 * for itself. The bot takes its input from an X server: it grabs global
 * hotkeys with XRecord and polls the X clipboard, and a compositor puts
 * neither within its reach. So the clipboard is fed in from `wl-paste`
 * instead, and the X hook -- which now sees the whole session rather than a
 * game that is not even an X client -- is taken back down.
 */
public final class Agent {

	/** Command line action, and the preference its hotkey is stored under. */
	private static final Map<String, String> ACTIONS = new LinkedHashMap<>();

	static {
		ACTIONS.put("increment", "hotkey_increment");
		ACTIONS.put("decrement", "hotkey_decrement");
		ACTIONS.put("reset", "hotkey_reset");
		ACTIONS.put("undo", "hotkey_undo");
		ACTIONS.put("redo", "hotkey_redo");
		ACTIONS.put("minimize", "hotkey_minimize");
		ACTIONS.put("alt-std", "hotkey_alt_std");
		ACTIONS.put("lock", "hotkey_lock");
		ACTIONS.put("boat", "hotkey_boat");
		ACTIONS.put("mod-360", "hotkey_mod_360");
		ACTIONS.put("aa-mode", "hotkey_toggle_aa_mode");
	}

	/** How long the bot is given to build the objects we inject into. */
	private static final long STARTUP_TIMEOUT_MILLIS = 60_000;

	private Agent() {
	}

	public static void premain(String arguments, Instrumentation instrumentation) {
		Thread thread = new Thread(Agent::run, "ninjabrain-bot-ipc");
		thread.setDaemon(true);
		thread.start();
	}

	private static void run() {
		Path path = socketPath();
		if (answers(UnixDomainSocketAddress.of(path))) {
			// Two bots would each take half of every copy, so stop this one
			// while it is still only a splash screen.
			log("another bot is already listening on " + path);
			System.exit(1);
		}
		try {
			serve(bind(path));
		} catch (Throwable error) {
			log("giving up: " + error);
		}
	}

	/**
	 * Whether the bot is on a session it cannot take its own input from.
	 *
	 * <p>`WAYLAND_DISPLAY` rather than `XDG_SESSION_TYPE`, because what
	 * decides this is whether `wl-paste` has a compositor to talk to.
	 */
	private static boolean isWaylandSession() {
		String display = System.getenv("WAYLAND_DISPLAY");
		return display != null && !display.isEmpty();
	}

	private static Path socketPath() {
		String runtimeDir = System.getenv("XDG_RUNTIME_DIR");
		return Path.of(runtimeDir == null ? "/tmp" : runtimeDir, "ninjabrain-bot.sock");
	}

	/**
	 * Listens on `path`, clearing away whatever a crash left behind. The
	 * caller has already established that nothing answers there.
	 */
	private static ServerSocketChannel bind(Path path) throws IOException {
		Files.deleteIfExists(path);
		ServerSocketChannel server = ServerSocketChannel.open(StandardProtocolFamily.UNIX);
		server.bind(UnixDomainSocketAddress.of(path));
		Runtime.getRuntime().addShutdownHook(new Thread(() -> {
			try {
				server.close();
				Files.deleteIfExists(path);
			} catch (IOException ignored) {
			}
		}));
		return server;
	}

	/**
	 * Whether a bot answers on `address`, which a socket lying around after a
	 * crash will not. Sends the empty request, so that the bot on the other
	 * end has something well-formed to reply to.
	 */
	private static boolean answers(UnixDomainSocketAddress address) {
		try (SocketChannel probe = SocketChannel.open(address)) {
			probe.shutdownOutput();
			return Channels.newInputStream(probe).readAllBytes().length != 0;
		} catch (IOException unanswered) {
			return false;
		}
	}

	private static void serve(ServerSocketChannel server) throws IOException, InterruptedException {
		ClipboardReader reader = null;
		if (isWaylandSession()) {
			// Connections that arrive before this returns wait in the backlog,
			// so nothing sent while the bot is starting is lost.
			reader = awaitClipboardReader();

			// Everything now comes in over the socket, and the hook would only
			// add ways to trigger the bot by accident: it records the whole X
			// session, so any hotkey pressed in any X client would fire.
			unhook();

			if (reader != null) {
				watchClipboard(reader);
			}
		}
		while (true) {
			try (SocketChannel connection = server.accept()) {
				handle(connection, reader);
			} catch (ClosedChannelException shuttingDown) {
				return;
			} catch (IOException error) {
				log("dropped a connection: " + error);
			}
		}
	}

	/**
	 * The bot's clipboard reader, once it exists, or null if it never appears.
	 *
	 * <p>It is built during startup and only ever handed to the keyboard
	 * listener, which is itself only built if the native hook registered. So
	 * the hook has to come up for the reader to be reachable, even though
	 * nothing afterwards has any use for it.
	 */
	private static ClipboardReader awaitClipboardReader() throws InterruptedException {
		long deadline = System.currentTimeMillis() + STARTUP_TIMEOUT_MILLIS;
		while (System.currentTimeMillis() < deadline) {
			ClipboardReader reader = ClipboardBridge.reader();
			if (reader != null) {
				return reader;
			}
			Thread.sleep(50);
		}
		log("the bot never built a clipboard reader, so F3+C will not arrive."
			+ " Its keyboard listener is what holds one, and that is only set up"
			+ " if JNativeHook registered -- look for a hook error above.");
		return null;
	}

	private static void unhook() {
		try {
			GlobalScreen.unregisterNativeHook();
		} catch (NativeHookException | UnsatisfiedLinkError error) {
			log("could not remove the native hook: " + error);
		}
	}

	/**
	 * Pushes every copy the compositor sees into the bot.
	 *
	 * <p>`wl-paste --watch` runs its command once per copy with the text on
	 * stdin, so the text is delimited on the way back out -- the alternative,
	 * setting the bot's own AWT clipboard, would make it the owner of the X
	 * selection and leave it copying back and forth with the compositor.
	 */
	private static void watchClipboard(ClipboardReader reader) throws IOException {
		Process watcher = new ProcessBuilder(
			"wl-paste", "--type", "text", "--watch", "sh", "-c", "cat; printf '\\000'")
			.redirectError(ProcessBuilder.Redirect.INHERIT)
			.start();
		Runtime.getRuntime().addShutdownHook(new Thread(watcher::destroy));

		Thread thread = new Thread(() -> {
			try (InputStream stream = watcher.getInputStream()) {
				ByteArrayOutputStream text = new ByteArrayOutputStream();
				for (int b = stream.read(); b != -1; b = stream.read()) {
					if (b != 0) {
						text.write(b);
					} else {
						ClipboardBridge.push(reader, text.toString(StandardCharsets.UTF_8));
						text.reset();
					}
				}
				log("wl-paste stopped watching the clipboard");
			} catch (IOException error) {
				log("stopped watching the clipboard: " + error);
			}
		}, "ninjabrain-bot-clipboard");
		thread.setDaemon(true);
		thread.start();
	}

	private static void handle(SocketChannel connection, ClipboardReader reader) throws IOException {
		// Read afresh every time, so that a hotkey rebound in the bot's own
		// options is picked up without a restart.
		Map<String, HotkeyPreference> hotkeys = HotkeyBridge.hotkeys();
		String request = new String(
			Channels.newInputStream(connection).readAllBytes(), StandardCharsets.UTF_8);
		int newline = request.indexOf('\n');
		String command = newline == -1 ? request : request.substring(0, newline);
		String body = newline == -1 ? "" : request.substring(newline + 1);

		String response;
		if (command.isEmpty()) {
			response = "";
		} else if (command.equals("list")) {
			response = String.join("\n", bound(hotkeys));
		} else if (command.equals("clipboard")) {
			if (reader == null) {
				response = "error: the bot has no clipboard reader";
			} else {
				ClipboardBridge.push(reader, body);
				response = "";
			}
		} else if (command.startsWith("run ")) {
			response = execute(command.substring(4).trim().split("\\s+"), hotkeys);
		} else {
			response = "error: no such command: " + command;
		}
		try (OutputStream out = Channels.newOutputStream(connection)) {
			out.write((response + "\n").getBytes(StandardCharsets.UTF_8));
		}
	}

	private static List<String> bound(Map<String, HotkeyPreference> hotkeys) {
		List<String> actions = new ArrayList<>();
		for (Map.Entry<String, String> action : ACTIONS.entrySet()) {
			if (hotkeys.containsKey(action.getValue())) {
				actions.add(action.getKey());
			}
		}
		return actions;
	}

	/** Fires each of `actions`, or none of them if any one is not bound. */
	private static String execute(String[] actions, Map<String, HotkeyPreference> hotkeys) {
		List<HotkeyPreference> firing = new ArrayList<>();
		for (String action : actions) {
			String preference = ACTIONS.get(action);
			if (preference == null) {
				return "error: no such action: " + action;
			}
			HotkeyPreference hotkey = hotkeys.get(preference);
			if (hotkey == null) {
				return "error: " + action + " has no hotkey";
			}
			firing.add(hotkey);
		}
		for (HotkeyPreference hotkey : firing) {
			hotkey.execute();
		}
		return "";
	}

	private static void log(String message) {
		System.err.println("ninjabrain-bot-ipc: " + message);
	}
}
