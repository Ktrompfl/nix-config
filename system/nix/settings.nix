{
  nix.settings = {
    experimental-features = [
      "nix-command"
      "flakes"
    ];

    # Would otherwise be `ask`, which stalls a non-interactive build waiting
    # for an answer. A flake does not get to add substituters by itself.
    accept-flake-config = false;

    # Keep the .drv and the build inputs of everything that is alive, so that
    # a direnv shell stays buildable offline and survives a gc.
    keep-outputs = true;

    # Build it here rather than give up when a substituter is unreachable or
    # only has some of what is needed.
    fallback = true;

    # A substituter that is down should cost seconds, not the five minutes
    # lix would otherwise spend. Lix takes this as an alias for
    # `max-connect-timeout`, which is the name it reports it under.
    connect-timeout = 5;

    http-connections = 50;

    # An emergency valve, not a policy: a build that would otherwise fill the
    # disk collects garbage until it has room again.
    min-free = 5 * 1024 * 1024 * 1024;
    max-free = 25 * 1024 * 1024 * 1024;
  };
}
