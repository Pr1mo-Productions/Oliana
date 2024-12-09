

fn main() -> Result<(), Box<dyn std::error::Error>>{

  // TODO abstract for win11 & other linux distros, this targets one of Arch's community packages.

  // "/opt/libtorch-cuda/lib/libtorch.so";
  println!("cargo:rustc-link-search=native=/opt/libtorch-cuda/lib");
  //println!("cargo:rustc-link-lib=torch");
  //println!("cargo:rustc-link-lib=torch_cuda");

  println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
  println!("cargo:rustc-link-arg=-Wl,--copy-dt-needed-entries");
  println!("cargo:rustc-link-arg=-ltorch");
  println!("cargo:rustc-link-arg=-ltorch_cuda");

  Ok(())
}
