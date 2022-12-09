{ allInOne ? true, incremental ? false, img_tag ? "" }:
self: super: {
  images = super.callPackage ./pkgs/images { inherit img_tag; };
  control-plane = super.callPackage ./pkgs/control-plane { inherit allInOne incremental; };
  openapi-generator = super.callPackage ./pkgs/openapi-generator { };
}
