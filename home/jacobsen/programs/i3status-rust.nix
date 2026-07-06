{ lib, ... }:
let
  # Only used as a data source for jay's own bar (see
  # ../wayland/jay-config/src/bar/i3status.rs): jay-config spawns this
  # process directly and reads its i3bar-protocol JSON on stdout. Rather
  # than picking a handful of placeholders and letting i3status-rust decide
  # formatting/severity/color, every block below renders *all* of its
  # non-icon placeholders as a literal JSON object (via i3status-rust's own
  # `\{`/`\}` brace-escaping) so jay-config gets the full picture and makes
  # every display/threshold/icon decision itself.
  #
  # `numFmt` strips the SI prefix and unit suffix `.eng()` would otherwise
  # add: `force_prefix`+`prefix:1` pins the scale to "no scaling" for every
  # unit alike (percent/bytes/hertz), since i3status-rust's `Unit::Percents`
  # always clamps back to `prefix:1` regardless of what's requested anyway.
  # This keeps every numeric value a plain, unit-less number jay can just
  # parse.
  numFmt = ".eng(w:15,hide_unit:true,prefix:1,force_prefix:true,hide_prefix:true)";

  # `$name` rendered as an always-present, quoted JSON string.
  num = name: ''"$'' + name + numFmt + ''"'';
  txt = name: ''"$'' + name + ''"'';

  # Same, but falling back to JSON `null` when the placeholder is absent -
  # i3status-rust's `{a|b}` group renders `a` if every placeholder in it
  # resolved, `b` otherwise.
  opt = name: "{" + num name + "|null}";
  optTxt = name: "{" + txt name + "|null}";

  field = key: value: ''"'' + key + ''":'' + value;
  obj = fields: ''\{'' + lib.concatStringsSep "," fields + ''\}'';

  # A literal, hand-picked tag (rather than a placeholder) for blocks whose
  # "state" is only observable through which of several format strings
  # i3status-rust chose to render (there's no separate placeholder for it).
  tagged = tag: fields: obj ([ (field "status" (''"'' + tag + ''"'')) ] ++ fields);
in
{
  programs.i3status-rust = {
    enable = true;

    bars.jay = {
      icons = "none";
      theme = "native";

      blocks = [
        {
          block = "cpu";
          interval = 2;
          format = obj [
            (field "utilization" (num "utilization"))
            (field "frequency" (opt "frequency"))
            (field "max_frequency" (opt "max_frequency"))
          ];
        }
        {
          block = "memory";
          interval = 5;
          format = obj [
            (field "mem_total" (num "mem_total"))
            (field "mem_free" (num "mem_free"))
            (field "mem_free_percents" (num "mem_free_percents"))
            (field "mem_avail" (num "mem_avail"))
            (field "mem_avail_percents" (num "mem_avail_percents"))
            (field "mem_total_used" (num "mem_total_used"))
            (field "mem_total_used_percents" (num "mem_total_used_percents"))
            (field "mem_used" (num "mem_used"))
            (field "mem_used_percents" (num "mem_used_percents"))
            (field "buffers" (num "buffers"))
            (field "buffers_percent" (num "buffers_percent"))
            (field "cached" (num "cached"))
            (field "cached_percent" (num "cached_percent"))
            (field "swap_total" (num "swap_total"))
            (field "swap_free" (num "swap_free"))
            (field "swap_free_percents" (num "swap_free_percents"))
            (field "swap_used" (num "swap_used"))
            (field "swap_used_percents" (num "swap_used_percents"))
            (field "zram_compressed" (num "zram_compressed"))
            (field "zram_decompressed" (num "zram_decompressed"))
            (field "zswap_compressed" (num "zswap_compressed"))
            (field "zswap_decompressed" (num "zswap_decompressed"))
          ];
        }
        {
          block = "disk_space";
          path = "/persist";
          interval = 30;
          info_type = "used";
          format = obj [
            (field "path" (txt "path"))
            (field "percentage" (num "percentage"))
            (field "total" (num "total"))
            (field "used" (num "used"))
            (field "available" (num "available"))
            (field "free" (num "free"))
          ];
        }
        {
          block = "sound";
          driver = "pipewire";
          device_kind = "sink";
          show_volume_when_muted = false;
          format = obj [
            (field "volume" (opt "volume"))
            (field "output_name" (txt "output_name"))
            (field "output_description" (txt "output_description"))
            (field "active_port" (optTxt "active_port"))
          ];
        }
        {
          block = "backlight";
          format = obj [ (field "brightness" (num "brightness")) ];
          missing_format = obj [ (field "brightness" "null") ];
        }
        {
          block = "battery";
          driver = "sysfs";
          interval = 10;
          format = tagged "discharging" [
            (field "percentage" (num "percentage"))
            (field "power" (opt "power"))
            (field "time_remaining" (opt "time_remaining"))
          ];
          charging_format = tagged "charging" [
            (field "percentage" (num "percentage"))
            (field "power" (opt "power"))
            (field "time_remaining" (opt "time_remaining"))
          ];
          full_format = tagged "full" [ (field "percentage" (num "percentage")) ];
          empty_format = tagged "empty" [ (field "percentage" (num "percentage")) ];
          not_charging_format = tagged "not_charging" [ (field "percentage" (num "percentage")) ];
          missing_format = tagged "missing" [ ];
        }
        {
          block = "net";
          interval = 5;
          format = tagged "up" [
            (field "device" (txt "device"))
            (field "ip" (optTxt "ip"))
            (field "ipv6" (optTxt "ipv6"))
            (field "ssid" (optTxt "ssid"))
            (field "frequency" (opt "frequency"))
            (field "signal_strength" (opt "signal_strength"))
            (field "bitrate" (opt "bitrate"))
            (field "speed_down" (num "speed_down"))
            (field "speed_up" (num "speed_up"))
            (field "nameserver" (optTxt "nameserver"))
          ];
          inactive_format = tagged "down" [ ];
          missing_format = tagged "missing" [ ];
        }
        {
          block = "notify";
          driver = "swaync";
          format = obj [
            (field "paused" ''{$paused"true"|"false"}'')
            (field "notification_count" (opt "notification_count"))
          ];
        }
        {
          block = "bluetooth";
          # Unlike the rest of these blocks, bluetooth tracks one specific
          # device rather than reporting adapter-wide state, and `mac` has
          # no default - deserializing the whole config.toml fails (taking
          # every other block down with it) if it's missing entirely, but
          # not if it's merely wrong, so this is a placeholder: find the
          # real address with `bluetoothctl devices` and replace it.
          mac = "00:00:00:00:00:00";
          format = tagged "connected" [ (field "percentage" (opt "percentage")) ];
          disconnected_format = obj [
            (field "status" ''"disconnected"'')
            (field "available" ''{$available"true"|"false"}'')
          ];
        }
      ];
    };
  };
}
