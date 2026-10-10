# Gitzi

Gitzi es un cliente de escritorio visual para trabajar con repositorios Git. Permite abrir, inicializar y clonar repositorios; revisar diffs por archivo; preparar o retirar archivos del índice; crear commits solo con los cambios preparados; y colaborar con remotos mediante Fetch, Pull y Push. También permite crear/cambiar ramas, configurar upstream, recorrer el historial paginado, consultar detalles de commits y comparar referencias. La recuperación cubre stash, merge, rebase, revert, resolución de conflictos, descarte confirmado y eliminación segura de ramas. El registro muestra las invocaciones reales de Git. Las operaciones se ejecutan en segundo plano.

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

En **Colaboración**, Fetch consulta los remotos configurados. Pull usa la política elegida en **Preferencias**: avance rápido únicamente, merge o rebase. Esa preferencia se guarda en la configuración local de Gitzi y no modifica la configuración global de Git. Push envía la rama actual a su upstream explícito, aunque `push.default` indique otra cosa, y nunca fuerza un rechazo. Para una rama sin upstream, publícala o asocia un upstream remoto existente. Las ramas, ahead/behind y fechas de fetch reflejan referencias locales conocidas y pueden estar desactualizadas hasta hacer Fetch.

Gitzi respeta los hooks, la firma y el resto de la configuración existente de Git. Reutiliza los credential helpers y la configuración SSH de Git; si la autenticación falla o se cancela, muestra el error original. No guarda credenciales. Para GitHub, clona una URL HTTPS o SSH o trabaja con un remoto GitHub ya configurado; la autenticación se completa con el credential helper o el agente SSH del sistema. Gitzi implementa las operaciones Git estándar, no una API de GitHub para pull requests o issues.

En **Integración y recuperación**, puedes crear, inspeccionar y aplicar stashes; eliminar un stash requiere confirmación. Merge, Rebase y Revert exigen un árbol limpio. Si aparece un conflicto, Gitzi conserva y muestra la operación, permite abrir el archivo en el editor externo configurado y marcarlo resuelto al prepararlo; después puedes continuar o abortar. Abort, descartar archivos y eliminar ramas muestran confirmaciones concretas. La eliminación de ramas no fuerza un borrado que Git rechaza.

Las inspecciones usan `git status --no-optional-locks`. Los repositorios recientes se guardan en la configuración del usuario: `%APPDATA%\Gitzi\recent-repositories.txt` en Windows o `$XDG_CONFIG_HOME/Gitzi/recent-repositories.txt` en Linux (por defecto `~/.config/Gitzi/recent-repositories.txt`).

## Comprobaciones de desarrollo

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- --deny warnings
cargo test
cargo run
```

La Entrega 4 se verificó en Windows con Rust 1.99.0 y Git real: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- --deny warnings` y 27 pruebas pasan. Los casos nuevos usan repositorios temporales y comprueban varios stashes/OID, merge y rebase con conflictos, continuación/aborto, revert, descarte de cambios preparados y sin preparar, renombres y rutas Unicode. No se validó una cuenta de proveedor real ni se pudo inspeccionar la ventana o probar teclado/foco porque las APIs de aplicaciones nativas no están disponibles. Linux y las mediciones de arranque, memoria y CPU siguen pendientes.
