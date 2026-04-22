{
  home-manager.users.jacobsen = {
    wayland.windowManager.jay.settings = {
      on-graphics-initialized = [
        # auto start frequently used apps
        {
          type = "exec";
          exec = "app2unit firefox";
        }
        {
          type = "exec";
          exec = "app2unit spotify";
        }
        {
          type = "exec";
          exec = "app2unit vesktop";
        }
      ];
    };
  };
}
