# Contratos compartidos

## Actualización T7: ventana compacta y publicación en GitHub

### Dominio y aplicación (`src/domain.rs`, `src/app.rs`, `src/main.rs`)

- `GitHubRepositoryVisibility::{Private, Public}` usa `Private` por defecto. `is_valid_github_repository_name` admite únicamente un slug ASCII de 1–100 caracteres alfanuméricos, `-`, `_` o `.`, excepto `.` y `..`; no admite owner, URL ni argumentos.
- `UiAction::StartGitHubBrowserLogin` inicia el flujo oficial web interactivo de GitHub CLI; `CheckGitHubAuth` solo consulta cuando se solicita.
- `UiAction::CreateGitHubRepository { name, visibility }` requiere confirmación de la UI. La aplicación valida que gh esté autenticado, exista repositorio abierto con commit y rama local, el slug sea válido y no exista `origin`.
- El runner recibe `GitOperation::CreateGitHubRepository { repository, name, visibility }` y, al tener éxito, devuelve `GitResult::GitHubRepositoryCreated { repository, remote_url }`. La aplicación presenta `success_message` y vuelve a inspeccionar estado, ramas y remotos.
- La acción se vuelve a validar tras la confirmación y el runner hace su propio preflight. Si la publicación terminó pero falla una consulta de refresco, la UI informa en un mensaje compuesto que el remoto se creó y que solo falló la actualización local.
- El efecto crea un repo en la cuenta gh activa, incorpora `origin` y publica los commits de la rama local actual. Los cambios sin commit no se suben. La visibilidad pública se identifica explícitamente; la opción privada es predeterminada. No se reemplaza ni reescribe un remoto existente. Un fallo de push puede dejar creado el repo remoto y debe indicarlo; no se intenta rollback automático.
- La ventana se inicia fija en 1024×700, sin redimensionamiento, maximización ni fullscreen. La UI debe seguir siendo funcional con scroll y distribución compacta.

### Runner (`src/git/runner.rs`, propiedad T7-GIT)

- El login visible usa `gh auth login --hostname github.com --web`, con argumentos separados y la consola necesaria para prompts restantes. La CLI abre el navegador; no se implementa OAuth propio ni se consulta/copiar tokens.
- Antes del efecto remoto revalida slug, autenticación `gh auth status --active --hostname github.com`, HEAD/rama local y ausencia de `origin`.
- Crea con `gh repo create <slug> --source . --remote origin --push --private|--public`, `cwd` igual al root y sin shell. Al éxito lee `git remote get-url origin` y devuelve la URL real.
- No hace commit automático, no sube cambios sin commit, no sobrescribe `origin`, no fuerza operaciones ni elimina recursos remotos. Los fallos posteriores a `gh repo create` explican que puede existir un repo remoto aunque no se haya completado el push/lectura del remote.
- Registra invocaciones reales y mantiene los límites y redacción existentes. Pruebas con repositorios temporales y argumentos falsos; no login/creación real ni red.

### Interfaz (`src/ui/**`, propiedad T7-UI)

- Presenta login explícitamente como iniciado mediante navegador, indicando el papel de la consola de GitHub CLI y ofreciendo verificación posterior.
- La publicación incluye nombre, privado/público (privado predeterminado), disponibilidad según autenticación/HEAD/slug/`origin` y confirmación con nombre, visibilidad, cuenta activa/remote `origin`, commits a publicar y aviso de que lo no confirmado seguirá local.
- Refina la jerarquía visual con estilo moderno ligero en 1024×700, sin retirar flujos D1–D6. El registro de comandos debe seguir plegable, permitir copiar y exponer salida/error por entrada en secciones plegables y acotadas.
- Presenta `success_message` y `error_message` como estados distintos, legibles y accesibles además del color.

Estado de la entrega: T7 integrada y revisada. `src/domain.rs`, `src/app.rs`, `src/main.rs` y los documentos raíz pertenecen al orquestador; runner y UI mantuvieron propietarios separados.

## Actualización T6: acceso a GitHub y presentación

### Dominio y aplicación (`src/domain.rs`, `src/app.rs`)

- `GitHubAuthStatus` expresa `Unknown`, `Authenticated`, `Unauthenticated`, `CliUnavailable` o `LoginStarted`; no contiene ni expone tokens.
- `UiAction::StartGitHubLogin` pide lanzar el flujo interactivo de GitHub CLI; `UiAction::CheckGitHubAuth` consulta el estado guardado por `gh`.
- `GitOperation::StartGitHubLogin` y `GitOperation::CheckGitHubAuth` son solicitudes sin repositorio. El runner devuelve `GitResult::GitHubLoginStarted` o `GitResult::GitHubAuthChecked { status }` y registra las invocaciones reales.
- La consulta de estado solo ocurre por `UiAction::CheckGitHubAuth`, al pulsar Verificar sesión; no consulta GitHub en cada arranque. Login y verificación usan el worker en segundo plano; el login solo confirma que abrió la consola. Al volver de la consola, el usuario pulsa Verificar para observar el resultado.
- `GitHubAuthStatus::CliUnavailable` es un estado visible y accionable; GitHub CLI es opcional para Gitzi y Git/Git Credential Manager/SSH siguen siendo válidos para remotos.

### Runner (`src/git/runner.rs`, propiedad T6-GIT)

- El acceso se inicia con `gh auth login --hostname github.com --web`, con argumentos separados y una consola interactiva visible. La CLI conserva sus prompts para que la persona elija HTTPS/SSH y si desea autenticar las operaciones Git. En Windows se abre una consola nueva; en Linux se usa un emulador de terminal disponible o se devuelve una instrucción concreta para ejecutarlo manualmente.
- No se usa shell, no se capturan entradas de la consola de login, no se consulta `gh auth token` y ningún token se agrega a salida, registro o estado de Gitzi. Las invocaciones de consulta son `gh auth status --active --hostname github.com` y no incluyen `--show-token`.
- La falta de `gh` genera `CliUnavailable` en la consulta, y un error al lanzar el flujo produce un error accionable. No se ejecuta `gh auth setup-git` ni se cambia configuración de Git silenciosamente.
- Solo se autentica github.com mediante GitHub CLI. Las operaciones Git existentes consumen credenciales estándar del usuario; Pull Requests/issues/API de GitHub permanecen fuera del alcance.

### Interfaz (`src/ui/**`, propiedad T6-UI)

- La franja superior incorpora el estado de sesión, el botón funcional **Iniciar sesión con GitHub** y **Verificar sesión**. Los estados `Unknown`, conectado, sin sesión, CLI ausente y consola abierta tienen etiquetas legibles además de señal visual.
- El texto aclara que se completa el flujo en la consola y se verifica al volver; la UI no solicita ni muestra tokens. Ambos controles respetan el estado ocupado.
- La ventana de inicio pasa a 1320×860 y conserva el cambio de tamaño; mínimo 960×640 para dar aire al contenido sin impedir usar ventanas compactas.
- Se conserva el resto de Entregas 1–5 y no se agregan controles sin acción.

Estado: contratos compartidos de Entregas 1–5 y de la actualización T6 descrita arriba. `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/domain.rs`, `src/app.rs`, `src/settings.rs` y este documento pertenecen al orquestador.

## Límites

- `git` ejecuta Git y convierte la salida real en tipos de dominio. No depende de egui.
- `app` coordina peticiones/respuestas, recientes y estado de vista.
- `ui` presenta `AppSnapshot` y emite `UiAction`; no inicia procesos ni interpreta salida.
- Los cambios de esta entrega en el repositorio abierto son solo consultas. Los casos de prueba mutantes usan repositorios temporales.

## Dominio: `src/domain.rs` (propiedad del orquestador)

- `RepositoryStatus { branch: String, upstream: Option<String>, ahead: u32, behind: u32, files: Vec<FileStatus> }`.
- `FileStatus { path: PathBuf, index_status: char, worktree_status: char, unmerged: bool }`. El estado original de dos caracteres porcelain se conserva; `unmerged` indica además el tipo de registro `u`, incluidos estados como `AA`/`DD` que no llevan `U`. `path` es relativa al root del repositorio y se preserva literalmente: no normalizar ni canonicalizar; todos los procesos de archivo usan `cwd` igual al root.
- `CommandStatus::{Succeeded, SucceededWithDifferences, Failed}`. La segunda variante significa que el proceso terminó correctamente y encontró diferencias; conserva el código de salida real, especialmente `1` en `diff --no-index`.
- `CommandEntry { id: u64, command_display: String, cwd: Option<PathBuf>, started_at: SystemTime, duration: Duration, status: CommandStatus, exit_code: Option<i32>, stdout: String, stderr: String }`.
- `AppSnapshot { git_checked: bool, git_version: Option<String>, active_repo: Option<PathBuf>, repository: Option<RepositoryStatus>, busy: bool, error_message: Option<String>, recent_repositories: Vec<PathBuf>, command_log: Vec<CommandEntry> }`.
- `UiAction::{OpenRepository(PathBuf), Refresh}`.

## Motor Git: `src/git/**` (propiedad de agente T1-GIT)

- `GitOperation::{DetectGit, InspectRepository(PathBuf)}`.
- `GitRequest { id: u64, generation: u64, operation: GitOperation }`.
- `GitResult::{GitDetected { version: String }, RepositoryInspected { path: PathBuf, status: RepositoryStatus }}`.
- `GitResponse { id: u64, generation: u64, result: Result<GitResult, GitError>, commands: Vec<CommandEntry> }`; `GitError { message: String }`.
- `GitWorker::spawn() -> GitWorker`, `submit(&self, GitRequest) -> Result<(), GitError>`, `try_recv(&self) -> Option<GitResponse>`.
- Un solo trabajador con cola acotada. Cada respuesta lleva id y generación. Todas las invocaciones, incluidos fallos de lanzamiento, generan registros fieles con directorio de trabajo, duración, salida y código de salida si existe. Sin shell concatenado.
- `DetectGit` ejecuta `git --version`. `InspectRepository(path)` ejecuta `git --no-optional-locks status --porcelain=v2 --branch -z --untracked-files=all` en `path`, con argumentos separados, rama/upstream/adelanto/atraso y estado de archivos. `--no-optional-locks` es obligatorio: `git status` puede refrescar y escribir el índice por defecto.
- Cada flujo de salida se limita a 1 MiB y el registro marca si se descartaron bytes. Una respuesta truncada de `status` o `--version` se rechaza; nunca se acepta ni se analiza como resultado completo. Los errores operativos se devuelven como `GitError`; no se analizan mensajes traducidos para determinar el estado.

## Interfaz: `src/ui/**` (propiedad de agente T1-UI)

- `UiState::default()` mantiene solo estado local de presentación.
- `render(ui: &mut egui::Ui, state: &mut UiState, snapshot: &AppSnapshot) -> Option<UiAction>`.
- Debe incluir detección de Git, campo para ruta, apertura, recientes, rama, upstream, estado de archivos, refresco, error/carga y registro plegable con copia.
- El registro se copia desde `CommandEntry::command_display`; el directorio de trabajo se muestra separado. No se crean comandos a partir de botones.
- Sin datos de repositorio ficticios ni acciones presentadas como funcionales que no lo sean.

## Actualización T8: UX compacta, terminal integrada y login sin consola

### Dominio y coordinación (propiedad del orquestador)

- Se conservan `AppSnapshot.command_log`, `CommandEntry`, `GitHubAuthStatus::LoginStarted` y `UiAction::StartGitHubBrowserLogin`. Se añade `CommandStatus::Running` y `GitResult::GitHubLoginFinished { status }` para reflejar el ciclo real del proceso sin guardar nuevos campos en el snapshot.
- `src/main.rs` inicia en 980×660 y conserva la ventana fija sin maximizar ni fullscreen.
- `src/app.rs` muestra la actividad de comandos en un `egui::Panel::bottom` fijo de egui y pasa el mismo snapshot a las funciones de UI indicadas abajo.

### Interfaz (propiedad T8-UI: solo `src/ui/**`)

- Conserva `render(ui, state, snapshot) -> Option<UiAction>` para el área principal y añade `render_terminal(ui, state, snapshot) -> ()` para el panel inferior.
- El panel inferior es una consola de solo lectura integrada en la ventana, no una pestaña ni una lista de tarjetas plegables. Presenta ejecuciones reales en orden temporal, hora, cada invocación y cwd, salidas stdout/stderr diferenciadas, estado/código y copia del comando. Se actualiza al recibir cada resultado del worker y sigue el final cuando el usuario ya está en el final; permite desplazarse por la sesión.
- Es un transcript de operaciones Git/gh de Gitzi, no un shell general. No inventa comandos ni presenta la salida de una ejecución como si fuese streaming por fragmentos mientras el proceso sigue activo.
- Botón **Iniciar sesión** activa la misma acción tipada; la UI explica que Gitzi abre el navegador externo del sistema sin terminal aparte, que el código de dispositivo se copia, y que después se usa **Verificar sesión**. No se pide token ni se presenta un WebView de GitHub.
- La ruta del repositorio y el destino de clonado ofrecen selección de carpetas mediante `rfd::FileDialog::pick_folder`; cancelar conserva el texto actual.
- Los controles interactivos usan objetivos de al menos 38 px de alto, buen contraste/foco y estados busy/deshabilitados; sin cambios a acciones de Git ni lógica de procesos.

### Motor Git (propiedad T8-GIT: solo `src/git/**`)

- `StartGitHubBrowserLogin` ejecuta `gh auth login --hostname github.com --web --clipboard` con argumentos separados, stdin/stdout/stderr no interactivos y sin ventana de terminal: `CREATE_NO_WINDOW` en Windows y los handles de stdio redirigidos a null en Linux.
- Al completar el spawn registra el comando como `Running` y devuelve `GitHubLoginStarted`; el worker envía esa respuesta antes de monitorizar el `Child` desde un hilo separado. Al terminar, emite `GitHubLoginFinished` con el estado y actualiza por ID la misma entrada con duración, código de salida y éxito/error. No captura las salidas del proceso ni bloquea la cola Git. `CheckGitHubAuth` sigue siendo explícito y se puede usar mientras el flujo de navegador esté en curso; una verificación iniciada durante el login no puede sobrescribir el resultado final de ese login. Al cerrar `GitWorker`, se cancela el watcher y se termina/recolecta el proceso para que no quede huérfano. La CLI realiza el flujo de dispositivo y abre el navegador del sistema.
- No pasa token, no vuelca el código de un solo uso en el registro, no espera en el worker, no cambia `git_protocol` ni configura `credential.helper` globalmente. El comando de inicio y fallos de lanzamiento se registran fielmente. La autenticación de `gh` no implica que el `git push` use el token de `gh`; Git conserva el helper/agente ya configurado por el usuario.
- Los tests de runner verifican argv del flujo y estados sin cuentas ni red real; no se prueba el login en una cuenta real.

## Integración: `src/app.rs` (propiedad del orquestador)

- Inicia el trabajador y agenda detección de Git; drena respuestas con `try_recv` sin bloquear egui.
- La generación actual evita que una respuesta de una selección previa reemplace el repositorio seleccionado después.
- Solo una inspección exitosa pasa a recientes y se persiste. No se escriben archivos del repositorio abierto.
- Registra resultados de ejecución reales; mantiene como máximo 200 entradas de historial en memoria.
- Al cambiar de repositorio, refrescar o cerrar no se envía ninguna operación mutante.

## Dependencia visual

Se fija eframe `0.36.2` tras verificar la documentación oficial. Se desactivan sus features por defecto (incluyen `wgpu`) y se habilitan `glow`, `default_fonts`, `x11` y `wayland`; la app selecciona `Renderer::Glow`. Esta es una primera opción para el prototipo, no una conclusión de rendimiento. eframe 0.36.2 requiere Rust 1.95. El flujo de App en esta versión requiere `App::ui` y permite `logic` antes de cada repintado.

## Entrega 2: cambios y commits

### Dominio compartido (`src/domain.rs`, propiedad T2-APP)

- `RepositoryStatus.has_head: Option<bool>` identifica si el repositorio ya tiene un commit; `None` significa que la respuesta no incluyó ese dato.
- El parser de porcelain v2 interpreta `# branch.oid (initial)` y un oid compuesto de ceros como `has_head=false`, un oid real como `true`, y el campo ausente como `None`. La forma observada con el Git de Windows de este host fue `(initial)`.
- `DiffArea::{Unstaged, Staged}` selecciona árbol de trabajo contra índice o índice contra HEAD.
- `FileDiff { repository, path, area, text, binary, truncated }` representa una diferencia de un archivo. Su identidad es repositorio + ruta relativa literal + área. La UI muestra el diff solo mientras esa identidad coincida con la selección actual; cambios de selección/repositorio y mutaciones invalidan la vista. Muestra estado explícito para binarios y salida truncada.
- `AppSnapshot.file_diff: Option<FileDiff>` contiene el diff vigente. Una respuesta obsoleta no reemplaza el diff actual.
- `UiAction` añadirá `InitializeRepository(PathBuf)`, `CloneRepository { source: String, destination: PathBuf }`, `LoadDiff { path: PathBuf, area: DiffArea }`, `StageFile(PathBuf)`, `UnstageFile(PathBuf)` y `Commit(String)`.

### Motor Git (`src/git/**`, propiedad T2-GIT)

- `GitOperation` añadirá `InitializeRepository { path }`, `CloneRepository { source, destination }`, `ReadDiff { repository, path, area, untracked }`, `StageFile { repository, path }`, `UnstageFile { repository, path, has_head }` y `Commit { repository, message }`. Cada operación de archivo recibe el repositorio explícito para establecer `cwd` sin depender de estado global.
- `GitResult` añadirá `RepositoryInitialized { path }`, `RepositoryCloned { path }`, `DiffLoaded { diff }`, `FileStaged { path }`, `FileUnstaged { path }` y `CommitCreated { path }`.
- Inicializará con `git init -- <path>` y clonará con `git clone -- <source> <destination>`. El destino no se borra ni se sobrescribe; Git determina si puede clonar ahí.
- Diff sin preparar: `git --no-optional-locks --literal-pathspecs diff --no-ext-diff --no-textconv --no-color -- <path>`. Diff preparado: misma consulta con `--cached`. Para un archivo untracked, genera el diff contra `/dev/null` con `git diff --no-index`; el código 1 que indica diferencias se registra como `SucceededWithDifferences`, no como error, conservando el código real. `/dev/null` es el operando nulo de Git también en Git para Windows.
- Cada flujo de diff se limita a 1 MiB. `binary` y `truncated` comunican el estado real; nunca se presenta salida incompleta como diff completo.
- Preparar afecta solo el archivo seleccionado, usando `git --literal-pathspecs add -- <path>`. Despreparar conserva el contenido del árbol de trabajo: con `has_head=true` usa `git --literal-pathspecs restore --staged -- <path>`; en una rama sin commits (`has_head=false`) usa `git --literal-pathspecs rm --cached -- <path>` para mantener el archivo en disco. Si `has_head` no se conoce, la app no ofrece esa acción. `--` separa opciones y ruta; `--literal-pathspecs` evita que Git interprete metacaracteres del nombre como patrones.
- Commit usa solo el índice y recibe el mensaje por stdin (`git commit --file=-`), sin incluirlo en argumentos ni en el comando copiable, y sin preparar cambios adicionales. El registro refleja argv/cwd y la salida real; la salida capturada puede contener texto emitido por Git o por un hook. El mensaje no forma parte de la invocación registrada. No desactiva hooks ni la firma configurada.
- Las operaciones se serializan en el worker existente. Tras init/clone y cada mutación, la app consulta de nuevo el estado. Rutas como argumentos separados y literales.
- Las pruebas mutantes y de flujo usan repositorios temporales. Cubren repo sin commits, binarios, nombres con espacios/Unicode, diffs staged y unstaged separados, stage/unstage y commit que conserva cambios no preparados.

### Interfaz (`src/ui/**`, propiedad T2-UI)

- Añadirá controles funcionales para inicializar, clonar, seleccionar archivo/área de diff, preparar/despreparar un archivo y crear un commit con mensaje.
- Las acciones incompatibles se deshabilitan mientras el worker está ocupado. Commit se habilita solo con mensaje no vacío y cambios preparados; un primer commit es válido.
- Un archivo parcialmente preparado puede mostrar a la vez el diff preparado y el no preparado. Esos diffs no se combinan.
- Identifica un repositorio sin commits y los binarios; informa si el diff está truncado.
- La UI no ejecuta procesos, no construye comandos y no muestra botones ficticios.

### Integración (`src/app.rs`, propiedad T2-APP)

- Valida acciones contra el repositorio y la lista de archivos vigentes. El estado de carga no bloquea egui.
- Tras stage/unstage/commit elimina el diff obsoleto y refresca el estado. Tras init/clone abre el destino y realiza la inspección inicial.
- Commit nunca prepara por su cuenta archivos no preparados. El estado usa `--no-optional-locks`; diffs y mutaciones constan como invocaciones reales en el registro.

## Entrega 3: colaboración

### Dominio y acciones (`src/domain.rs`, propiedad del orquestador)

- `RepositoryStatus.last_fetch_at: Option<SystemTime>` es la fecha local de modificación de `FETCH_HEAD`; no se calcula una consulta remota ni se inventa una fecha si el archivo no existe.
- `BranchInfo { name, kind, current, upstream }`, con `BranchKind::{Local, Remote}`, y `RemoteInfo { name }` representan las ramas y remotos que Git conoce localmente.
- `CommitSummary { id, author, authored_at, subject }`, `CommitFileChange { status, path }` y `CommitDetail { summary, message, changed_files }` representan una página de historial y el detalle cargado bajo demanda.
- `RefComparison { base_ref, target_ref, commits_ahead, commits_behind, diff, truncated }` compara dos refs: ahead cuenta commits alcanzables desde target que no están en base; behind cuenta commits alcanzables desde base que no están en target.
- `AppSnapshot` añade `branches`, `remotes`, `history`, `history_has_more`, `commit_detail` y `ref_comparison`. La app invalida datos que pertenecen a una selección/repo anterior.
- `UiAction` añade `Fetch`, `Pull`, `Push`, `PublishBranch(String)`, `SetUpstream { local_branch, remote_branch }`, `CreateBranch(String)`, `SwitchBranch(String)`, `LoadMoreHistory`, `LoadCommitDetail(String)` y `CompareReferences { base_ref, target_ref }`.

### Motor Git (`src/git/**`, propiedad T3-GIT)

- Añade `GitOperation::{ListBranches(PathBuf), CreateBranch { repository, name }, SwitchBranch { repository, name }, SetUpstream { repository, local_branch, remote_branch }, Fetch(PathBuf), PullFastForward(PathBuf), Push { repository, local_branch, upstream }, PublishBranch { repository, remote, local_branch }, LoadHistory { repository, offset, limit }, LoadCommitDetail { repository, commit_id }, CompareReferences { repository, base_ref, target_ref }}`.
- Añade resultados homónimos: `BranchesListed { repository, branches, remotes }`, `BranchCreated { repository }`, `BranchSwitched { repository }`, `UpstreamSet { repository }`, `Fetched { repository }`, `Pulled { repository }`, `Pushed { repository }`, `BranchPublished { repository }`, `HistoryLoaded { repository, offset, commits, has_more }`, `CommitLoaded { repository, detail }` y `ReferencesCompared { repository, comparison }`.
- El mínimo de Git documentado para esta entrega es 2.23 por el uso de `git switch`; la interfaz muestra el error real si Git no soporta un comando.
- `ListBranches` consulta referencias y remotos locales con formatos estructurados. La app agenda esa consulta tras abrir repo o cambiar rama. Historial se pagina con máximo 50 registros por página; la app solicita 30 y `has_more` se calcula leyendo como máximo un registro adicional.
- Crear rama crea y cambia a la nueva rama desde HEAD; cambiar rama usa la rama existente y deja que Git rechace cambios incompatibles. `SetUpstream` solo asocia una rama local con una referencia remota que ya existe. `PublishBranch` publica la rama actual en el remoto elegido y establece upstream en la misma operación.
- `Fetch` consulta todos los remotos configurados. `PullFastForward` ejecuta pull con `--ff-only`, política que la interfaz identifica y el motor aplica explícitamente sin cambiar la configuración global de Git. `Push` recibe la rama local actual y upstream observados, valida su OID y lo envía al destino remoto upstream mediante un refspec explícito; no hereda refspecs múltiples de `push.default` ni `remote.<name>.push`. Publicar también usa el OID validado de la rama actual y, solo si el push termina con éxito, configura `branch.<local>.remote` y `branch.<local>.merge` con `git config --local`. Push y publicación desactivan `remote.<name>.mirror` solo para la invocación y usan `--no-follow-tags`, sin alterar preferencias persistentes ni ampliar refs. Ninguna operación usa `--force`, `--force-with-lease` ni reintenta un push rechazado. Si no hay upstream, la app exige publicar la rama, configurar un upstream existente o cancelar la acción. Publicar y hacer push requieren una rama local actual con al menos un commit.
- En Fetch/Pull/Push/Publicar se conserva la configuración de credenciales y SSH existente. Solo para estas invocaciones `GIT_TERMINAL_PROMPT=0` evita esperar por entrada en una consola oculta; los helpers configurados siguen habilitados y sus fallos o cancelaciones se devuelven como `GitError` con la salida real redactada. No se solicitan ni guardan credenciales.
- La fecha de fetch se obtiene del mtime local de la ruta que `git rev-parse --git-path FETCH_HEAD` devuelve, si existe, después de Fetch/Pull y al inspeccionar el repo. No representa un sondeo remoto.
- Historial usa salida delimitada por NUL y referencias commit; detalles cargan mensaje y lista de archivos bajo demanda. Comparación valida cada referencia como commit antes de usar sus OID y limita el diff a 1 MiB. Las referencias inválidas son errores operativos, no argumentos de opciones.
- Todos los resultados mantienen `GitResponse { id, generation, ... }`, el worker serial existente, argv separados, cwd explícito, registro real y límites de salida. Mutaciones solo ocurren por acción directa y se prueban en repositorios temporales con remoto bare temporal.

### Interfaz (`src/ui/**`, propiedad T3-UI)

- Presenta ramas locales/remotas, creación/cambio de rama, upstream, fetch, pull FF-only y push/publicación explícitos; las acciones incompatibles se deshabilitan con el worker ocupado o sin los datos requeridos.
- Muestra ahead/behind y `last_fetch_at` como conocimiento local, sin insinuar frescura remota. Presenta historia paginada, detalle de commit y comparación de dos ramas/referencias disponibles.
- Errores/cancelación de autenticación se muestran desde el resultado real del motor. La UI no crea comandos, elige credenciales ni convierte un push rechazado en otra acción.

### Integración (`src/app.rs`, propiedad del orquestador)

- Valida acciones usando repo, rama local actual y listas vigentes; publicar/push exigen una rama local actual con commit. Un cambio de repo invalida listas, historial, detalle y comparación.
- Tras crear/cambiar rama, publicar, configurar upstream, fetch o pull, refresca estado de repo y referencias locales. Tras push refresca contadores y el estado de la rama. La paginación añade resultados solo para el repositorio y offset actuales. La UI solo presenta una comparación mientras coincide con las dos referencias seleccionadas.
- El registro conserva argv y salidas reales, aplica la redacción existente y muestra los errores del helper de credenciales cancelado/fallido. No se persiste salida ni secreto.

## Entrega 4: integración y recuperación

### Dominio y acciones (`src/domain.rs`, propiedad del orquestador)

- `IntegrationKind::{Merge, Rebase, Revert}` identifica la operación en curso en `RepositoryStatus.integration`. Los conflictos se derivan de los archivos porcelain no fusionados; no se mantienen dos fuentes de estado.
- `PullPolicy::{FastForwardOnly, Merge, Rebase}` tiene `FastForwardOnly` como valor predeterminado. Se persiste en preferencias de Gitzi, no en la configuración global de Git.
- `StashEntry { oid, selector, message, created_at }` y `StashDetail { oid, diff, truncated }` representan la lista e inspección bajo demanda. Acciones mutantes llevan OID, no un `stash@{n}` obsoleto; el motor vuelve a resolverlo antes de operar.
- `AppSnapshot` añade `stashes`, `stash_detail`, `pull_policy` y `external_editor`. `external_editor` es una ruta opcional al ejecutable, persistida sin argumentos de shell.
- `settings::Preferences { pull_policy, external_editor }` tiene `load_preferences() -> Preferences` y `save_preferences(&Preferences) -> Result<(), String>`. Se almacena en `preferences.txt` junto a recientes, con lectura tolerante y versión explícita; no reescribe `recent-repositories.txt`.
- `UiAction` añade `SetPullPolicy(PullPolicy)`, `SetExternalEditor(Option<PathBuf>)`, `OpenConflictFile(PathBuf)`, `CreateStash { message: Option<String>, include_untracked: bool }`, `LoadStashDetail(String)`, `ApplyStash(String)`, `DropStash(String)`, `MergeBranch(String)`, `RebaseBranch(String)`, `RevertCommit(String)`, `ContinueIntegration`, `AbortIntegration`, `DiscardFile(PathBuf)` y `DeleteBranch(String)`.

### Motor Git (`src/git/**`, propiedad T4-GIT)

- Sustituye `PullFastForward(PathBuf)` por `Pull { repository, policy }`. Ejecuta `pull --ff-only`, `pull --no-rebase` o `pull --rebase` según la preferencia cargada; no cambia configuración persistente de Git.
- Añade `ListStashes(PathBuf)`, `CreateStash { repository, message, include_untracked }`, `LoadStashDetail { repository, stash_oid }`, `ApplyStash { repository, stash_oid }`, `DropStash { repository, stash_oid }`, `MergeBranch { repository, branch }`, `RebaseBranch { repository, branch }`, `RevertCommit { repository, commit_id }`, `ContinueIntegration(PathBuf)`, `AbortIntegration(PathBuf)`, `DiscardFile { repository, path, has_head }`, `DeleteBranch { repository, name }` y `OpenConflictFile { repository, path, editor }`.
- Añade `StashesListed { repository, stashes }`, `StashCreated { repository }`, `StashDetailLoaded { repository, detail }`, `StashApplied { repository }`, `StashDropped { repository }`, `IntegrationCompleted { repository }`, `IntegrationContinued { repository }`, `IntegrationAborted { repository }`, `FileDiscarded { repository }`, `BranchDeleted { repository }` y `ExternalFileOpened { repository }`. Toda respuesta conserva `id`, `generation`, registro real y errores redactados.
- Inspecciona `MERGE_HEAD`, `rebase-merge`, `rebase-apply` y `REVERT_HEAD` mediante rutas que Git resuelve; prioridad rebase, merge y revert. No deduce una operación activa únicamente del texto de error.
- `ContinueIntegration` y `AbortIntegration` solo ejecutan el subcomando que corresponda a la operación detectada y la vuelven a validar justo antes. No reintentan ni abortan automáticamente.
- Rebase/Merge/Revert requieren una rama local con commit, estado limpio y ninguna integración en curso. No se fuerza ninguna operación. Merge y Revert conservan hooks y firma; Pull con Merge/Rebase puede terminar en conflicto y devuelve estado real.
- Crear stash conserva cambios staged y unstaged; `include_untracked` controla archivos untracked. Aplicar no elimina el stash. Eliminar resuelve el OID seleccionado al selector vigente y usa `git stash drop`, sin borrar reflogs directamente.
- `DiscardFile` elimina únicamente cambios del archivo confirmado por el usuario: restaura tracked desde HEAD en índice y árbol de trabajo; elimina untracked por pathspec literal; en un repo sin HEAD limpia la entrada del índice y el archivo exacto. Rechaza conflictos. `DeleteBranch` usa borrado no forzado y rechaza rama actual/no fusionada.
- `OpenConflictFile` inicia el ejecutable configurado mediante `Command` con argumentos separados y ruta absoluta del archivo; no usa shell ni interpreta comandos del repositorio. Registra el lanzamiento y devuelve error visible si no puede abrirlo.
- Las pruebas mutantes usan repositorios temporales y remoto bare. Cubren stash, merge/rebase/revert, conflictos, continuación/aborto, rutas literales, borrado confirmado y estados parciales.

### Integración e interfaz (`src/app.rs` y `src/ui/**`)

- Tras inspeccionar y después de mutaciones/refrescos, la app carga estado de operación y lista de stashes. Tras error de Pull, stash apply, merge, rebase, revert, continuar o abortar, inspecciona de nuevo y conserva el error original.
- StageFile en un conflicto es la vía para marcar ese archivo resuelto. Continuar se habilita cuando no quedan archivos sin resolver; Abort se limita a una integración detectada.
- La UI muestra política Pull persistida, stash, integración/conflictos, editor externo y confirmaciones que identifican archivo, stash o rama. Operaciones que borran trabajo o referencias requieren confirmar en un segundo paso; branch delete nunca usa force.
- Selecciones y resultados de stash/detalle se invalidan al cambiar de repo, refrescar o mutar. Los datos de la UI nunca sustituyen la validación de ramas, OID o conflictos que hace el motor.

## Entrega 5: pulido y distribución

### Interfaz (`src/ui/**`, propiedad T5-UI)

- Puede reorganizar componentes y definir el tema dentro de `src/ui/**`; no cambia `domain`, acciones, comandos, contratos de estado ni lógica de Git.
- Conserva todos los flujos aceptados de Entregas 1–4. No presenta botones sin acción ni sustituye datos reales por ejemplos.
- Centraliza colores, tipografía y espaciado; conserva etiquetas y señales además del color para comunicar estados. Mantiene contraste legible, foco y operación por teclado en los flujos principales.
- Hace adaptables paneles y listas a ventanas pequeñas. Diferencias y listas deben respetar límites/truncamiento del dominio y mantener visibles los estados vacíos, ocupados y de error.
- Evita repintado continuo o animaciones decorativas. La revisión visual manual solo se declara si se pudo hacer en un host gráfico.

### Distribución y CI (`.github/workflows/**`, `scripts/package-windows.ps1`, `README.md`, `.gitignore`, propiedad T5-DIST)

- El workflow no publica releases ni necesita secretos. Usa permisos `contents: read`, se ejecuta en push/PR y puede lanzarse manualmente.
- La matriz de Windows y Linux instala las bibliotecas nativas requeridas, fija la toolchain compatible y ejecuta `cargo fmt --all -- --check`, `cargo clippy --all-targets -- --deny warnings`, `cargo test` y build release.
- El workflow empaqueta binarios de Windows y Linux como artefactos de ejecución de Actions; no los publica como releases. El script PowerShell genera un ZIP reproducible de Windows desde el binario release e incluye instrucciones de ejecución/dependencia de Git.
- Los paquetes no contienen Git: requieren un `git` disponible en PATH y respetan la configuración/credenciales locales del usuario. El README diferencia CI configurada de una ejecución remota observada.
- No se afirma que GitHub esté conectado ni que el workflow haya corrido hasta que el usuario configure el remoto y haga push.

### Verificación e integración (propiedad del orquestador)

- Tras integrar ambos frentes, repetir formato, Clippy, pruebas y build release en Windows; ejecutar el script de empaquetado y revisar el ZIP.
- Revisar estáticamente el workflow y el empaquetado Linux. Compatibilidad/GUI de Linux solo se afirma si se ejecutan en Linux; una definición de CI todavía no equivale a una ejecución de Actions.
- Registrar tamaños y, si el host permite medirlos de forma fiable, arranque/uso en reposo. No declarar inspección visual manual sin haber abierto la ventana en un host gráfico.
