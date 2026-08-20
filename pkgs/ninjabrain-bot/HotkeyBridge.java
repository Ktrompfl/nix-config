package ninjabrainbot.io.preferences;

import java.util.LinkedHashMap;
import java.util.Map;

/**
 * The seam the bot's hotkeys offer to anything in their own package.
 *
 * <p>Every {@link HotkeyPreference} adds itself to a public static list as it
 * is constructed, and firing one is a plain method call -- the keyboard
 * listener does nothing but match a key event against the list and call
 * {@link HotkeyPreference#execute()} on whatever matched. Calling it directly
 * skips the matching, and with it the key event, the X server and the keycode
 * arithmetic that recovering one would need.
 *
 * <p>Which preference is which is only recorded in the key its code is stored
 * under, which is package-private.
 */
public final class HotkeyBridge {

	private HotkeyBridge() {
	}

	/** The bound hotkeys, by the preference key they are stored under. */
	public static Map<String, HotkeyPreference> hotkeys() {
		Map<String, HotkeyPreference> bound = new LinkedHashMap<>();
		for (HotkeyPreference hotkey : HotkeyPreference.hotkeys) {
			if (hotkey.getCode() != -1) {
				bound.put(stripSuffix(hotkey.code.key), hotkey);
			}
		}
		return bound;
	}

	private static String stripSuffix(String codeKey) {
		return codeKey.endsWith("_code") ? codeKey.substring(0, codeKey.length() - 5) : codeKey;
	}
}
