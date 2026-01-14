{
  pkgs,
  ...
}:

{
  packages = with pkgs; [
    nixfmt
    treefmt
    cargo-nextest
    cargo-tarpaulin
    cargo-insta
    cargo-dist
    cargo-audit
  ];

  languages = {
    rust = {
      enable = true;
      channel = "stable";
      components = [
        "rustc"
        "cargo"
        "clippy"
        "rustfmt"
        "rust-analyzer"
        "rust-src"
      ];
    };
    nix = {
      enable = true;
      lsp.package = pkgs.nil;
    };
  };

  treefmt = {
    enable = true;
    config = {
      programs.nixfmt = {
        enable = true;
        package = pkgs.nixfmt-rfc-style;
      };
      programs.rustfmt = {
        enable = true;
      };
      programs.taplo = {
        enable = true;
      };
    };
  };

  git-hooks.hooks = {
    treefmt.enable = true;
    clippy.enable = true;
    commitizen.enable = true;
    nextest = {
      enable = true;
      name = "cargo-nextest";
      description = "Run tests with cargo-nextest";
      entry = "${pkgs.cargo-nextest}/bin/cargo-nextest nextest run";
      pass_filenames = false;
      stages = [ "pre-commit" ];
    };
  };
}
