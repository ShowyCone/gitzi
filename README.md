# Gitzi

Gitzi es un cliente de escritorio visual para trabajar con repositorios Git. Permite abrir, inicializar y clonar repositorios; revisar diffs por archivo; preparar o retirar archivos del índice; crear commits solo con los cambios preparados; y colaborar con remotos mediante Fetch, Pull y Push. También permite crear/cambiar ramas, configurar upstream, recorrer el historial paginado, consultar detalles de commits y comparar referencias. El registro muestra las invocaciones reales de Git. Las operaciones se ejecutan en segundo plano.

## Requisitos

- Git 2.23 o posterior, disponible en `PATH` (la aplicación usa `git switch`).
- Rust 1.95 o posterior con Cargo. eframe 0.36.2 requiere Rust 1.95.
- Identidad de autor/committer configurada en Git para crear commits.
- En Linux, las bibliotecas de desarrollo requeridas por eframe para X11/Wayland, OpenGL y XKB. Linux sigue pendiente de validación en un host correspondiente.

## Ejecutar

Desde la raíz del proyecto:

```sh
cargo run
```

En Gitzi, escribe la ruta de un repositorio local existente y pulsa **Abrir**, o usa **Inicializar** o **Clonar** para crear/obtener uno. Selecciona un archivo y su área (**Sin preparar** o **Preparado**) para cargar el diff. Los botones **Preparar** y **Deshacer preparación** solo afectan el archivo elegido. **Crear commit** usa exclusivamente el índice y conserva cambios sin preparar.

En **Colaboración**, Fetch consulta los remotos configurados. Pull usa avance rápido únicamente; si las historias divergen, Gitzi muestra el error sin iniciar un merge o rebase. La estrategia FF-only es fija en Entrega 3; su preferencia persistida y las opciones explícitas de merge o rebase quedan para Entrega 4. Push envía la rama actual a su upstream explícito, aunque `push.default` indique otra cosa, y nunca fuerza un rechazo. Para una rama sin upstream, publícala o asocia un upstream remoto existente. Las ramas, ahead/behind y fechas de fetch reflejan referencias locales conocidas y pueden estar desactualizadas hasta hacer Fetch. Gitzi reutiliza los credential helpers y la configuración SSH existentes; si la autenticación falla o se cancela, muestra el error de Git. No guarda credenciales.

Gitzi respeta los hooks, la firma y el resto de la configuración existente de Git. La interfaz identifica Pull como **avance rápido** y no cambia la política global de Git.

Las inspecciones usan `git status --no-optional-locks`. Los repositorios recientes se guardan en la configuración del usuario: `%APPDATA%\Gitzi\recent-repositories.txt` en Windows o `$XDG_CONFIG_HOME/Gitzi/recent-repositories.txt` en Linux (por defecto `~/.config/Gitzi/recent-repositories.txt`).

## Comprobaciones de desarrollo

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- --deny warnings
cargo test
cargo run
```

La Entrega 3 se verificó en Windows con Rust 1.99.0 y Git real: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- --deny warnings` y 18 pruebas pasaron. La prueba de colaboración usa dos clones y un remoto bare temporal, cubre Push con configuraciones Git que normalmente amplían el alcance, el rechazo de una rama divergente, Pull fast-forward, historial y comparación. No se validó una cuenta de proveedor real ni se pudo inspeccionar la ventana o probar teclado/foco porque las APIs de aplicaciones nativas no estaban disponibles. Linux y las mediciones de arranque, memoria y CPU siguen pendientes.
