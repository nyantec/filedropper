{
  description = "filedropper";

  outputs = { self, nixpkgs }: let
    version = self.shortRev or (toString self.lastModifiedDate);

    archForPkgFilename = targetPlatform: {
      i686 = "x86-32";
    }.${targetPlatform.parsed.cpu.arch} or targetPlatform.parsed.cpu.arch;

    overlay = final: prev: {
      filedropper = final.callPackage (
        { rustPlatform }:

        rustPlatform.buildRustPackage {
          pname = "filedropper";
          inherit version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
        }
      ) {};

      filedropper-pkg = final.callPackage (
        { stdenv, filedropper, zstd, runCommand }:

        runCommand "filedropper-pkg" {
          nativeBuildInputs = [ zstd ];
        } ''
          mkdir -p usr/bin $out
          cp ${filedropper}/bin/filedropper usr/bin
          tar --zstd -cf $out/filedropper-${archForPkgFilename stdenv.targetPlatform}-SNAPSHOT.pkg usr
        ''
      ) {};
    };
    pkgs = import nixpkgs {
      system = "x86_64-linux";
      crossSystem = {
        isStatic = true;
        config = "x86_64-unknown-linux-musl";
      };
      overlays = [ overlay ];
    };
    pkgs32 = import nixpkgs {
      system = "x86_64-linux";
      crossSystem = {
        isStatic = true;
        config = "i686-unknown-linux-musl";
        rustc.config = "i586-unknown-linux-musl";
      };
      overlays = [ overlay ];
    };
  in {
    inherit overlay;
    packages.x86_64-linux = {
      filedropper-x86_64 =     pkgs.filedropper;
      filedropper-x86_64-pkg = pkgs.filedropper-pkg;
      filedropper-i686 =       pkgs32.filedropper;
      filedropper-i686-pkg =   pkgs32.filedropper-pkg;
    };
    defaultPackage.x86_64-linux = self.packages.x86_64-linux.filedropper-x86_64-pkg;
  };
}
