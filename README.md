# Gitzi

Gitzi es un cliente de escritorio visual para trabajar con repositorios Git. La Entrega 2 permite abrir, inicializar y clonar repositorios; revisar diffs por archivo; preparar o retirar archivos del índice; y crear commits solo con los cambios preparados. El registro muestra las invocaciones reales de Git. Las operaciones se ejecutan en segundo plano.

## Requisitos

- Git instalado y disponible en `PATH`.
- Rust 1.95 o posterior con Cargo. eframe 0.36.2 requiere Rust 1.95.
- Identidad de autor/committer configurada en Git para crear commits.
- En Linux, las bibliotecas de desarrollo requeridas por eframe para X11/Wayland, OpenGL y XKB. Linux sigue pendiente de validación en un host correspondiente.

## Ejecutar

Desde la raíz del proyecto:

```sh
cargo run
```

En Gitzi, escribe la ruta de un repositorio local existente y pulsa **Abrir**, o usa **Inicializar** o **Clonar** para crear/obtener uno. Selecciona un archivo y su área (**Sin preparar** o **Preparado**) para cargar el diff. Los botones **Preparar** y **Deshacer preparación** solo afectan el archivo elegido. **Crear commit** usa exclusivamente el índice y conserva cambios sin preparar. Gitzi respeta los hooks, la firma y la configuración existentes de Git.

Las inspecciones usan `git status --no-optional-locks`. Los repositorios recientes se guardan en la configuración del usuario: `%APPDATA%\Gitzi\recent-repositories.txt` en Windows o `$XDG_CONFIG_HOME/Gitzi/recent-repositories.txt` en Linux (por defecto `~/.config/Gitzi/recent-repositories.txt`).

## Comprobaciones de desarrollo

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- --deny warnings
cargo test
cargo run
```

La Entrega 2 se verificó en Windows con Rust 1.99.0 y Git real: formato, Clippy y 17 pruebas pasaron; `cargo run` compiló e inició la aplicación. No se pudo inspeccionar la ventana ni probar teclado/foco porque las APIs de aplicaciones nativas no estaban disponibles en la sesión. Linux y las mediciones de arranque, memoria y CPU siguen pendientes.
