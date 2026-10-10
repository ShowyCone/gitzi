# Gitzi

Cliente de escritorio visual para consultar repositorios Git. La Entrega 1 detecta Git, abre un repositorio existente, muestra rama/estado y registra las consultas reales. Las operaciones de esta entrega son de solo lectura.

## Requisitos

- Git instalado y disponible en `PATH`.
- Rust 1.95 o posterior con Cargo. eframe 0.36.2 requiere Rust 1.95.
- En Linux, las bibliotecas de desarrollo requeridas por eframe para X11/Wayland, OpenGL y XKB.

## Ejecutar

Desde la raíz del proyecto:

```powershell
cargo run
```

En Windows PowerShell y Linux el comando es el mismo. Al abrir Gitzi, escribe la ruta de un repositorio local existente y pulsa **Abrir**. Las inspecciones usan `git status` y no modifican el repositorio. Los recientes se guardan en la configuración del usuario: `%APPDATA%\Gitzi\recent-repositories.txt` en Windows o `$XDG_CONFIG_HOME/Gitzi/recent-repositories.txt` en Linux (por defecto `~/.config/Gitzi/recent-repositories.txt`).

## Comprobaciones de desarrollo

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run
```

La compilación y las pruebas de Windows y Linux aún deben ejecutarse en hosts con la toolchain instalada. Las dependencias nativas exactas de Linux dependen del entorno de escritorio y se documentarán tras probar el empaquetado.
