{
  # What happens to a window when it is mapped.
  wayland.windowManager.jay.settings.windows = [
    {
      name = "wl-mirror";
      match = {
        app-id = "at.yrlf.wl_mirror";
        just-mapped = true;
      };
      auto-focus = false;
      action = [
        {
          type = "move-to-workspace";
          name = "0";
        }
        {
          type = "move-to-output";
          workspace = "0";
          output.name = "beamer";
        }
        "enter-fullscreen"
      ];
    }
    # {
    #   match = {
    #     types = "client-window";
    #     just-mapped = true;
    #     workspace = "0";
    #   };
    # }
    {
      # dwindle / spiral layout
      match = {
        types = "client-window";
        just-mapped = true;
      };
      action = [
        "split-major"
      ];
    }
  ];
}
