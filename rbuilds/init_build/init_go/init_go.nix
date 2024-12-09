{ buildGoModule
, ...
}: buildGoModule {
  name = "eif-init";
  src = ./.;

  vendorHash = null;

  CGO_ENABLED = 0;
  ldflags = [ "-s" "-w" ];
}
