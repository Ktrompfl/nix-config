package ninjabrainbot.io;

/**
 * The seam the bot's clipboard reader offers to anything in its own package.
 *
 * <p>{@link ClipboardReader} polls the X clipboard on a thread of its own and,
 * whenever it changes, pushes the text into an observable that the coordinate
 * parser is subscribed to. Pushing into that observable directly is
 * indistinguishable from a copy, and does not involve X at all.
 *
 * <p>The reader is not reachable from anywhere public: the only lasting
 * reference to it is the one {@link KeyboardListener} keeps, and that listener
 * only exists once the native hook has registered.
 */
public final class ClipboardBridge {

	private ClipboardBridge() {
	}

	/** The bot's clipboard reader, or null while the bot is still starting. */
	public static ClipboardReader reader() {
		KeyboardListener listener = KeyboardListener.instance;
		return listener == null ? null : listener.clipboardReader;
	}

	/** Hands `text` to the bot as though it had just been copied. */
	public static void push(ClipboardReader reader, String text) {
		// The reader truncates what it reads from X, so match it: the parsers
		// only ever look at a single line of F3+C output anyway.
		reader.clipboardString.set(text.length() > 1000 ? text.substring(0, 1000) : text);
	}
}
