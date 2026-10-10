# Gitzi

Gitzi es un cliente de escritorio visual para trabajar con repositorios Git. Permite abrir, inicializar y clonar repositorios; revisar diffs por archivo; preparar o retirar archivos del índice; crear commits solo con los cambios preparados; y colaborar con remotos mediante Fetch, Pull y Push. También permite crear/cambiar ramas, configurar upstream, recorrer el historial paginado, consultar detalles de commits y comparar referencias. La recuperación cubre stash, merge, rebase, revert, resolución de conflictos, descarte confirmado y eliminación segura de ramas. El registro muestra las invocaciones reales de Git. Las operaciones se ejecutan en segundo plano.

## Requisitos

- Para ejecutar un paquete: Git 2.23 o posterior, disponible en `PATH` (Gitzi usa `git switch`), y una sesión gráfica de escritorio.
- Para compilar desde el código: Rust 1.99.0 con Cargo. El `Cargo.lock` fija las dependencias.
- En Linux, el sistema debe proporcionar las bibliotecas de ejecución de su sesión gráfica (X11 o Wayland) y OpenGL. La CI instala las bibliotecas de desarrollo para compilar, pero estas no se incluyen en el paquete.
- Para crear commits, configura nombre y correo de autor/committer en Git.
- GitHub CLI (`gh`) es opcional; se requiere únicamente para usar el botón de inicio de sesión de GitHub. Las funciones de Git siguen usando los credential helpers o el agente SSH configurados en el sistema.

## Ejecutar

Desde la raíz del proyecto:

```sh
cargo run
```

En Gitzi, escribe la ruta de un repositorio local existente y pulsa **Abrir**, o usa **Inicializar** o **Clonar** para crear/obtener uno. Selecciona un archivo y su área (**Sin preparar** o **Preparado**) para cargar el diff. Los botones **Preparar** y **Deshacer preparación** solo afectan el archivo elegido. **Crear commit** usa exclusivamente el índice y conserva cambios sin preparar.

La ventana inicia en 1320×860 y puede reducirse hasta 960×640. En la franja superior, **Iniciar sesión con GitHub** abre el flujo web interactivo de GitHub CLI en una consola. Completa allí la autenticación; al volver, pulsa **Verificar sesión**. Si usarás remotos HTTPS, elige HTTPS y acepta autenticar Git con GitHub en los prompts de `gh`; para SSH, elige SSH y utiliza una clave/agente configurado.

En **Colaboración**, Fetch consulta los remotos configurados. Pull usa la política elegida en **Preferencias**: avance rápido únicamente, merge o rebase. Esa preferencia se guarda en la configuración local de Gitzi y no modifica la configuración global de Git. Push envía la rama actual a su upstream explícito, aunque `push.default` indique otra cosa, y nunca fuerza un rechazo. Para una rama sin upstream, publícala o asocia un upstream remoto existente. Las ramas, ahead/behind y fechas de fetch reflejan referencias locales conocidas y pueden estar desactualizadas hasta hacer Fetch.

Gitzi respeta los hooks, la firma y el resto de la configuración existente de Git. Reutiliza los credential helpers y la configuración SSH de Git; si la autenticación falla o se cancela, muestra el error original. No guarda credenciales ni captura tokens durante el login. Para GitHub, clona una URL HTTPS o SSH o trabaja con un remoto GitHub ya configurado; la sesión se completa con GitHub CLI y Git usa la credencial configurada en sus prompts, el credential helper o el agente SSH del sistema. Si `gh` no está instalado, puedes seguir usando Gitzi con las credenciales de Git ya configuradas. La interfaz solo inicia y verifica la sesión; Gitzi implementa las operaciones Git estándar, no una API de GitHub para pull requests o issues. Consulta [instalación de GitHub CLI](https://github.com/cli/cli#installation) y [su guía de autenticación](https://cli.github.com/manual/gh_auth_login).

En **Integración y recuperación**, puedes crear, inspeccionar y aplicar stashes; eliminar un stash requiere confirmación. Merge, Rebase y Revert exigen un árbol limpio. Si aparece un conflicto, Gitzi conserva y muestra la operación, permite abrir el archivo en el editor externo configurado y marcarlo resuelto al prepararlo; después puedes continuar o abortar. Abort, descartar archivos y eliminar ramas muestran confirmaciones concretas. La eliminación de ramas no fuerza un borrado que Git rechaza.

Las inspecciones usan `git status --no-optional-locks`. Los repositorios recientes se guardan en la configuración del usuario: `%APPDATA%\Gitzi\recent-repositories.txt` en Windows o `$XDG_CONFIG_HOME/Gitzi/recent-repositories.txt` en Linux (por defecto `~/.config/Gitzi/recent-repositories.txt`).

## Comprobaciones de desarrollo

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- --deny warnings
cargo test
cargo run
```

Entrega 4 se verificó localmente en Windows con Rust 1.99.0 y Git real: formato, Clippy y 27 pruebas pasan. Los casos usan repositorios temporales y cubren stash/OID, merge y rebase con conflictos, continuación/aborto, revert, descarte de cambios preparados y sin preparar, renombres y rutas Unicode. La validación local de Entrega 5 se registra en `ESTADO_DEL_PROYECTO.md`.

## Empaquetado

En Windows x86_64, instala Rust 1.99.0 o posterior y ejecuta desde la raíz:

```powershell
./scripts/package-windows.ps1
```

El script compila en modo release con `Cargo.lock` y escribe `dist/gitzi-windows-x86_64.zip`. El ZIP contiene `gitzi.exe` e instrucciones en `README.txt`; no incluye Rust ni Git. Para Linux, compila en un host Linux con las bibliotecas de desarrollo de eframe instaladas y distribuye el ejecutable `target/release/gitzi` junto con instrucciones y las bibliotecas gráficas requeridas por el sistema. La CI genera un `.tar.gz` para Linux y un ZIP para Windows como artefactos descargables de la ejecución, retenidos durante 14 días; no crea una publicación o release.

## GitHub y autenticación

Gitzi se conecta a GitHub mediante comandos Git estándar y el remoto configurado en el repositorio. Para habilitar la CI de este proyecto, crea un repositorio GitHub vacío y configura su URL como `origin` (usa la URL real de tu repositorio):

```sh
git remote add origin https://github.com/OWNER/REPOSITORY.git
git push -u origin main
```

Si `origin` ya existe, actualiza su URL con `git remote set-url origin URL` en vez de agregarlo de nuevo. Sustituye `main` por la rama actual si tiene otro nombre. El primer push requiere que completes la autenticación HTTPS con tu credential helper, o uses una URL SSH y una clave/agente SSH configurado. Después del push, GitHub Actions ejecutará el workflow en push; los pull requests y el disparo manual también están configurados. Hasta conectar el remoto y observar una ejecución, la CI remota no está validada. El workflow solo solicita permiso de lectura de contenido, no requiere secretos y no publica releases.

Gitzi no implementa una API de GitHub ni gestiona pull requests o issues. `gh` guarda la autenticación en la ubicación segura disponible en el sistema (con fallback documentado por GitHub CLI si no encuentra almacén seguro); Gitzi no accede al token. Los paquetes no contienen Git, GitHub CLI ni credenciales.
