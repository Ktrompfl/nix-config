{
  # Workaround: G502 Lightspeed (046d:407f, via receiver 046d:c539) scrolls 1/8
  # as far per notch after system suspend (`jay seat-test`: 15/120 instead of
  # 120/120 per notch).
  #
  # Cause (read from hid-logitech-hidpp.c / hid-logitech-dj.c, linux 7.3.1):
  # - hi_res_scroll_enable() puts the mouse into hi-res mode and sets the
  #   kernel's wheel multiplier to 8. The mouse drops hi-res mode when it loses
  #   power or link, the kernel is not told and keeps dividing by 8.
  # - The kernel re-enables hi-res mode only on a connect event
  #   (hidpp_connect_event), on probe, or via HIDPP_QUIRK_RESET_HI_RES_SCROLL
  #   (set for 0x407f), which fires only on a battery online 0 -> 1 transition.
  # - None of these happen across system suspend: hidpp has no resume hook,
  #   logi_dj only has reset_resume (re-enables DJ mode on the receiver, not
  #   its devices), the kernel never saw the battery go offline, and no
  #   connect notification arrives.
  #
  # Proper fix (upstream, linux-input): on system resume, run the connect
  # handling (or reset_hi_res_work) for paired devices, likely by having
  # logi_dj propagate resume to its children; ideally query the wheel mode
  # instead of assuming it.
  #
  # Rebinding the driver re-probes the device, which re-enables hi-res mode.
  # Drop this once the kernel handles it.
  powerManagement.resumeCommands = ''
    drv=/sys/bus/hid/drivers/logitech-hidpp-device
    for dev in "$drv"/*:046D:407F.*; do
      [ -e "$dev" ] || continue
      id=''${dev##*/}
      echo -n "$id" > "$drv/unbind"
      echo -n "$id" > "$drv/bind"
    done
  '';
}
