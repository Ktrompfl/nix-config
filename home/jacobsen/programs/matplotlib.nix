{ config, lib, ... }:
{
  # matplotlibrc treats `#` as a comment, so colours are bare hex.
  xdg.config.files."matplotlib/matplotlibrc" = {
    generator = lib.generators.toKeyValueLines { separator = ": "; };

    value = with config.theme.colors.withoutHashtag; {
      backend = "QtAgg";

      "boxplot.flierprops.color" = "white";
      "boxplot.flierprops.markeredgecolor" = base03;
      "boxplot.boxprops.color" = base03;
      "boxplot.whiskerprops.color" = base03;
      "boxplot.capprops.color" = base03;
      "boxplot.medianprops.color" = base02;

      "font.serif" =
        "Source Code Pro, DejaVu Serif, Bitstream Vera Serif, Computer Modern Roman, New Century Schoolbook, Century Schoolbook L, Utopia, ITC Bookman, Bookman, Nimbus Roman No9 L, Times New Roman, Times, Palatino, Charter, serif";
      "font.sans-serif" =
        "Source Code Pro, DejaVu Sans, Bitstream Vera Sans, Computer Modern Sans Serif, Lucida Grande, Verdana, Geneva, Lucid, Arial, Helvetica, Avant Garde, sans-serif";
      "font.cursive" =
        "Source Code Pro, Apple Chancery, Textile, Zapf Chancery, Sand, Script MT, Felipa, Comic Neue, Comic Sans MS, cursive";
      "font.fantasy" = "Source Code Pro, Chicago, Charcoal, Impact, Western, Humor Sans, xkcd, fantasy";
      "font.monospace" =
        "Source Code Pro, DejaVu Sans Mono, Bitstream Vera Sans Mono, Computer Modern Typewriter, Andale Mono, Nimbus Mono L, Courier New, Courier, Fixed, Terminal, monospace";

      "text.color" = base05;

      "axes.facecolor" = base00;
      "axes.edgecolor" = base0F;
      "axes.grid" = "True";
      "axes.grid.axis" = "y";
      "axes.labelcolor" = base03;
      "axes.axisbelow" = "True";
      "axes.spines.left" = "True";
      "axes.spines.bottom" = "True";
      "axes.spines.top" = "False";
      "axes.spines.right" = "False";
      "axes.prop_cycle" =
        "cycler('color', ['${base08}', '${base09}', '${base0A}', '${base0B}', '${base0C}', '${base0D}'])";

      "xtick.color" = base03;
      "ytick.color" = base03;
      "grid.color" = base02;

      "figure.facecolor" = base00;
      "figure.edgecolor" = base0F;
      "savefig.facecolor" = base00;
    };
  };
}
