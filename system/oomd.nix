{
  # Without user slices oomd only watches the system, which is the half that is
  # never under pressure here. With them it kills out of app-graphical.slice
  # first, leaving the compositor and its daemons alone.
  systemd.oomd = {
    enable = true;
    enableUserSlices = true;
  };

  systemd.user.slices = {
    # A browser leak should throttle rather than push the session into swap.
    app-graphical.sliceConfig = {
      MemoryHigh = "70%";
      ManagedOOMMemoryPressure = "kill";
      ManagedOOMMemoryPressureLimit = "50%";
    };

    # The compositor and its bar keep priority when something pins every core.
    session-graphical.sliceConfig = {
      CPUWeight = 300;
      IOWeight = 300;
      ManagedOOMMemoryPressure = "auto";
    };

    background-graphical.sliceConfig = {
      CPUWeight = 30;
      IOWeight = 30;
      ManagedOOMMemoryPressure = "kill";
      ManagedOOMMemoryPressureLimit = "40%";
    };
  };
}
