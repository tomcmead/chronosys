use aya_build::{Package, Toolchain};

fn main() -> anyhow::Result<()> {
    // Hardcoded package layout rather than manually querying cargo_metadata
    let ebpf_package = Package {
        name: "chronosys-ebpf",
        root_dir: "../chronosys-ebpf",
        ..Default::default()
    };

    // Compile eBPF package
    aya_build::build_ebpf([ebpf_package], Toolchain::default())
}
