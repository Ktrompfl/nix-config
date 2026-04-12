{
  home-manager.users.jacobsen = {
    wayland.windowManager.jay.settings = {
      on-graphics-initialized = [
        # auto start frequently used apps
        {
          type = "exec";
          exec = "firefox";
        }
        {
          type = "exec";
          exec = "spotify";
        }
        {
          type = "exec";
          exec = "vesktop";
        }
      ];
    };
  };
}
