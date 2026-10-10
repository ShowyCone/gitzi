# Contratos compartidos

Estado: contrato compartido para Entregas 1 y 2. `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/domain.rs`, `src/app.rs`, `src/settings.rs` y este documento pertenecen al orquestador.

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
