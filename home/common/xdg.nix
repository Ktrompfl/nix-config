{ config, ... }:
{
  xdg = {
    cache.directory = "${config.storage.state}/.local/cache";
    data.directory = "${config.storage.state}/.local/share";
    state.directory = "${config.storage.state}/.local/state";
  };

  preservation.preserveAt.state-dir.directories = [
    ".local/cache"
    ".local/share"
    ".local/state"
  ];

  # Programs that would otherwise make a directory of their own in ~, and
  # have no module of their own here to say so.
  environment.sessionVariables = {
    NODE_REPL_HISTORY = "${config.xdg.state.directory}/node_repl_history";
    NPM_CONFIG_CACHE = "${config.xdg.cache.directory}/npm";
    NPM_CONFIG_USERCONFIG = "${config.xdg.config.directory}/npm/npmrc";
  };
}
