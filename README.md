# Gitzi

Gitzi es un cliente de escritorio visual para trabajar con repositorios Git. Permite abrir, inicializar y clonar repositorios; revisar diffs por archivo; preparar o retirar archivos del índice; crear commits solo con los cambios preparados; y colaborar con remotos mediante Fetch, Pull y Push. También permite crear/cambiar ramas, configurar upstream, recorrer el historial paginado, consultar detalles de commits y comparar referencias. La recuperación cubre stash, merge, rebase, revert, resolución de conflictos, descarte confirmado y eliminación segura de ramas. Con GitHub CLI puede iniciar sesión desde el navegador y crear un repositorio remoto privado o público desde los commits de la rama actual. El registro muestra las invocaciones reales de Git y GitHub CLI. Las operaciones se ejecutan en segundo plano.

## Requisitos

- Para ejecutar un paquete: Git 2.23 o posterior, disponible en `PATH` (Gitzi usa `git switch`), y una sesión gráfica de escritorio.
- Para compilar desde el código: Rust 1.99.0 con Cargo. El `Cargo.lock` fija las dependencias.
- En Linux, el sistema debe proporcionar las bibliotecas de ejecución de su sesión gráfica (X11 o Wayland) y OpenGL. La CI instala las bibliotecas de desarrollo para compilar, pero estas no se incluyen en el paquete.
- El selector nativo de carpetas usa el portal XDG del entorno de escritorio en Linux; si el portal no responde, la biblioteca puede recurrir a Zenity. Windows abre el diálogo de carpetas nativo del Explorador.
- Para crear commits, configura nombre y correo de autor/committer en Git.
- GitHub CLI (`gh`) es opcional; se requiere para iniciar sesión desde Gitzi o crear un repositorio GitHub. Las funciones Git estándar siguen usando los credential helpers o el agente SSH configurados en el sistema.

## Ejecutar

Desde la raíz del proyecto:

```sh
cargo run
```

En Gitzi, escribe la ruta de un repositorio local existente y pulsa **Abrir**, o usa **Inicializar** o **Clonar** para crear/obtener uno. Selecciona un archivo y su área (**Sin preparar** o **Preparado**) para cargar el diff. Los botones **Preparar** y **Deshacer preparación** solo afectan el archivo elegido. **Crear commit** usa exclusivamente el índice y conserva cambios sin preparar.

La ventana inicia fija en 980×660; no se puede redimensionar, maximizar ni poner en pantalla completa. En la franja superior, **Iniciar sesión** inicia el flujo web oficial de GitHub CLI sin abrir otra consola. GitHub CLI abre el navegador del sistema y copia al portapapeles un código temporal; completa allí la aprobación y pulsa **Verificar sesión** al volver. La aprobación sigue siendo externa porque GitHub no se integra en una página embebida de Gitzi.

En **Colaboración**, Fetch consulta los remotos configurados. Pull usa la política elegida en **Preferencias**: avance rápido únicamente, merge o rebase. Esa preferencia se guarda en la configuración local de Gitzi y no modifica la configuración global de Git. Push envía la rama actual a su upstream explícito, aunque `push.default` indique otra cosa, y nunca fuerza un rechazo. Para una rama sin upstream, publícala o asocia un upstream remoto existente. Las ramas, ahead/behind y fechas de fetch reflejan referencias locales conocidas y pueden estar desactualizadas hasta hacer Fetch.

Gitzi respeta los hooks, la firma y el resto de la configuración existente de Git. El login guarda la autenticación según la configuración de `gh`, pero no cambia el protocolo ni el credential helper global de Git. Las operaciones Git estándar reutilizan los credential helpers y la configuración SSH ya existentes; si la autenticación de Git falla o se cancela, Gitzi muestra el error original. Gitzi no lee tokens. La autenticación de `gh` no garantiza por sí sola que el proceso `git push` use el token de GitHub CLI. Si `gh` no está instalado, puedes seguir usando las funciones Git con las credenciales ya configuradas, pero no las opciones GitHub de la UI. Consulta [instalación de GitHub CLI](https://github.com/cli/cli#installation) y [su guía de autenticación](https://cli.github.com/manual/gh_auth_login).

En un repositorio local con al menos un commit y sin remoto `origin`, **Crear repositorio desde commits** permite elegir un nombre y visibilidad. Privado está seleccionado inicialmente. Tras confirmar, Gitzi crea el repositorio en la cuenta activa de `gh`, añade `origin` y publica los commits de la rama local actual. Los cambios sin commit permanecen locales. No se sobrescribe un `origin` existente ni se crean commits automáticamente. Si se elige Público, Gitzi lo advierte antes de publicar. Un error al publicar puede dejar creado el repositorio remoto; Gitzi no intenta borrarlo automáticamente. Esta opción usa `gh repo create`; Gitzi no implementa API propia, pull requests o issues.

En **Integración y recuperación**, puedes crear, inspeccionar y aplicar stashes; eliminar un stash requiere confirmación. Merge, Rebase y Revert exigen un árbol limpio. Si aparece un conflicto, Gitzi conserva y muestra la operación, permite abrir el archivo en el editor externo configurado y marcarlo resuelto al prepararlo; después puedes continuar o abortar. Abort, descartar archivos y eliminar ramas muestran confirmaciones concretas. La eliminación de ramas no fuerza un borrado que Git rechaza.

Las inspecciones usan `git status --no-optional-locks`. El transcript inferior muestra las invocaciones reales y sus salidas en orden; permite copiar cada comando y conservar el desplazamiento durante nuevas operaciones. No ejecuta comandos ingresados como texto. Los selectores de carpeta abren diálogos nativos del sistema para elegir repo, inicialización y destino de clonado. Los repositorios recientes se guardan en la configuración del usuario: `%APPDATA%\Gitzi\recent-repositories.txt` en Windows o `$XDG_CONFIG_HOME/Gitzi/recent-repositories.txt` en Linux (por defecto `~/.config/Gitzi/recent-repositories.txt`).

## Comprobaciones de desarrollo

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- --deny warnings
cargo test
cargo run
```

Entrega 4 se verificó localmente en Windows con Rust 1.99.0 y Git real: formato, Clippy y 27 pruebas pasan. Los casos usan repositorios temporales y cubren stash/OID, merge y rebase con conflictos, continuación/aborto, revert, descarte de cambios preparados y sin preparar, renombres y rutas Unicode. La validación local de Entrega 5 se registra en `ESTADO_DEL_PROYECTO.md`.

La actualización T7 se verificó en Windows con Rust 1.99.0: formato y Clippy sin warnings, 33 pruebas y build release pasan. La revisión de código fue independiente. El login real, la publicación contra una cuenta GitHub, la GUI visual y Linux siguen pendientes, como se detalla en `ESTADO_DEL_PROYECTO.md`.

La actualización T8 se verificó en Windows con formato, `git diff --check`, Clippy con `--deny warnings`, 38 pruebas y build release. La revisión independiente final no encontró bloqueos. No se abrió la GUI ni se usó una cuenta GitHub real; los límites constan en `ESTADO_DEL_PROYECTO.md`.

## Empaquetado

En Windows x86_64, instala Rust 1.99.0 o posterior y ejecuta desde la raíz:

```powershell
./scripts/package-windows.ps1
```

El script compila en modo release con `Cargo.lock` y escribe `dist/gitzi-windows-x86_64.zip`. El ZIP contiene `gitzi.exe` e instrucciones en `README.txt`; no incluye Rust ni Git. Para Linux, compila en un host Linux con las bibliotecas de desarrollo de eframe instaladas y distribuye el ejecutable `target/release/gitzi` junto con instrucciones y las bibliotecas gráficas requeridas por el sistema. La CI genera un `.tar.gz` para Linux y un ZIP para Windows como artefactos descargables de la ejecución, retenidos durante 14 días; no crea una publicación o release.

## GitHub y autenticación

Gitzi puede crear el remoto desde **Crear repositorio desde commits**, siempre que el directorio tenga commits, no tenga `origin` y la sesión de GitHub CLI esté verificada. Para habilitar la CI de este proyecto, puedes usar esa opción o crear un repositorio vacío y configurar su URL como `origin` (usa la URL real de tu repositorio):

```sh
git remote add origin https://github.com/OWNER/REPOSITORY.git
git push -u origin main
```

Si `origin` ya existe, actualiza su URL con `git remote set-url origin URL` en vez de agregarlo de nuevo. Sustituye `main` por la rama actual si tiene otro nombre. El primer push requiere que completes la autenticación HTTPS con tu credential helper, o uses una URL SSH y una clave/agente SSH configurado. Después del push, GitHub Actions ejecutará el workflow en push; los pull requests y el disparo manual también están configurados. Hasta conectar el remoto y observar una ejecución, la CI remota no está validada. El workflow solo solicita permiso de lectura de contenido, no requiere secretos y no publica releases.

`gh` guarda la autenticación en la ubicación segura disponible en el sistema (con fallback documentado por GitHub CLI si no encuentra almacén seguro); Gitzi no accede al token. Los paquetes no contienen Git, GitHub CLI ni credenciales. El flujo con una cuenta GitHub real requiere que el usuario complete el login desde su equipo; no se realiza durante las pruebas automatizadas.
