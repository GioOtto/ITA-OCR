use std::path::Path;

fn main() {
    // `tauri_build::build()` dichiara come dipendenze solo tauri.conf.json e
    // capabilities. L'interfaccia in ../ui viene incorporata nel binario da
    // `generate_context!` al momento della compilazione del crate, ma cargo
    // non sa che quei file la riguardano: modificando soltanto app.js o
    // styles.css la compilazione non riparte e si ottiene un eseguibile con
    // dentro ancora la vecchia interfaccia, senza nessun errore che lo dica.
    dichiara_sorgenti(Path::new("../ui"));
    // Risorse PE di Windows e icona predefinita della finestra: una modifica
    // deve rieseguire tauri-build anche senza modifiche al codice Rust.
    dichiara_sorgenti(Path::new("icons"));

    tauri_build::build()
}

/// Registra ricorsivamente i file e le directory come input di build.
fn dichiara_sorgenti(dir: &Path) {
    println!("cargo:rerun-if-changed={}", dir.display());
    let Ok(voci) = std::fs::read_dir(dir) else {
        return;
    };
    for voce in voci.flatten() {
        let percorso = voce.path();
        if percorso.is_dir() {
            dichiara_sorgenti(&percorso);
        } else {
            println!("cargo:rerun-if-changed={}", percorso.display());
        }
    }
}
