# Tareas por entrega

## Entrega 1 — cerrada

| ID | Responsable | Alcance | Archivos | Dependencias | Criterios | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| T1-APP | Orquestador | Manifiesto, dominio, coordinación, configuración local y conexión de módulos | `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/domain.rs`, `src/app.rs`, `src/settings.rs`, docs raíz | Arquitectura leída; contratos en `CONTRATOS.md` | Compila, inicia, detecta Git en segundo plano, coordina acciones, persiste recientes y no escribe en el repo abierto | aceptada |
| T1-GIT | Agente de motor | Worker Git, consulta de detección/estado y registro real | `src/git/**` | Contrato fijado en `CONTRATOS.md`; recibe tipos ya definidos | Argumentos separados, cwd explícito, `--no-optional-locks`, consulta porcelain de solo lectura, salida acotada con truncamiento señalado/rechazado, respuesta no bloqueante; pruebas del repo temporal e índice sin cambios | aceptada |
| T1-UI | Agente de interfaz | Vista operativa de Entrega 1 | `src/ui/**` | Contrato fijado en `CONTRATOS.md`; snapshot y acciones ya definidos | Español; apertura por ruta, recientes, rama/upstream/estado, carga/error, registro plegable y copiar; refresco deshabilitado sin repo; sin lógica de proceso ni botones falsos | aceptada |
| T1-VERIFY | Agente de verificación | Revisión independiente del flujo integrado y criterios | Sin cambios de producción | Integración coherente de T1-APP/T1-GIT/T1-UI | Inspección, compilación, formato, Clippy y 9 pruebas en Windows; limitaciones visuales y de rendimiento documentadas | aceptada |

## Entrega 2 — cambios y commits

| ID | Responsable | Alcance | Archivos | Dependencias | Criterios | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| T2-APP | Orquestador | Tipos de dominio, coordinación, estado de vista y documentación | `src/domain.rs`, `src/app.rs`, `src/settings.rs`, `README.md`, docs raíz | Contrato T2 en `CONTRATOS.md`; respuestas de T2-GIT y acciones de T2-UI | Init/clone; revisar por área; stage/unstage; commit solo del índice; descartar resultados de diff obsoletos; refrescar sin bloquear | aceptada |
| T2-GIT | Agente de motor | Init/clone, diff por archivo/área, stage/unstage y commit | `src/git/**` | Contrato T2 en `CONTRATOS.md`; dominio T2 en `src/domain.rs` | Argumentos separados, cwd explícito y pathspec literal; diffs limitados con estado binario/truncado; HEAD inicial/conflictos; registro real; pruebas con repos temporales | aceptada |
| T2-UI | Agente de interfaz | Flujo revisar → preparar → commit e init/clone | `src/ui/**` | Contrato T2 en `CONTRATOS.md`; snapshot y acciones T2 | Español; diff staged/unstaged separado e identidad repo/path/área; controles funcionales; binarios, truncamiento, HEAD inicial/conflicto; commit preserva mensaje exacto | aceptada |
| T2-VERIFY | Agente verificador | Revisión independiente de Entrega 2 integrada | Sin cambios de producción | Integración T2-APP/T2-GIT/T2-UI | 17 pruebas pasan en host autorizado; revisión Git temporal de paths literales, borrado, diff untracked; limitación del sandbox, visual y Linux documentada; sin defectos funcionales confirmados | aceptada |

## Entrega 3 — colaboración

| ID | Responsable | Alcance | Archivos | Dependencias | Criterios | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| T3-APP | Orquestador | Integración de acciones, datos compartidos, actualización de estado y documentos | `src/domain.rs`, `src/app.rs`, `src/settings.rs`, `README.md`, docs raíz | Contrato T3 definido en `CONTRATOS.md` | Flujo integrado para sincronización segura, ramas, upstream, historial paginado, detalle y comparación; respuestas obsoletas ignoradas | aceptada |
| T3-GIT | Agente de motor | Operaciones remotas, ramas, upstream, historial y comparación | `src/git/**` | Tipos y comandos exactos definidos en `CONTRATOS.md`; dominio T3 ya disponible | Git real con bare remoto temporal; Push/Publish usan OID validado y refspec explícito; publicación configura upstream tras éxito; auth conserva configuración; ningún push forzado; salidas acotadas | aceptada |
| T3-UI | Agente de interfaz | Controles de colaboración e historia en español | `src/ui/**` | Snapshot/acciones T3 en `src/domain.rs` y `CONTRATOS.md` | Operaciones solicitadas disponibles con estados vacíos/carga/error; upstream ausente explícito; historial, detalle y comparación; acciones limitadas a refs actuales | aceptada |
| T3-VERIFY | Agente verificador | Revisión independiente del flujo integrado | Sin cambios de producción | Integración T3-APP/T3-GIT/T3-UI coherente | 18 pruebas pasan con Git real; dos clones intercambian cambios con remoto bare; push rechazado nunca fuerza; revisión de ramas, upstream, paginación y comparación; limitaciones documentadas | aceptada |

Entrega 4 completa la preferencia persistida de Pull y las opciones explícitas de integración que faltaban tras D3.

## Entrega 4 — integración y recuperación

| ID | Responsable | Alcance | Archivos | Dependencias | Criterios | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| T4-APP | Orquestador | Tipos, preferencias persistidas, validación, coordinación y documentación | `src/domain.rs`, `src/app.rs`, `src/settings.rs`, docs raíz | Contrato T4 en `CONTRATOS.md` | Política Pull persistida; respuestas obsoletas ignoradas; errores parciales refrescan estado | aceptada |
| T4-GIT | Agente de motor | Stash, integración, conflictos, descarte y editor externo | `src/git/runner.rs` | Contrato T4 y tipos de dominio fijados por T4-APP | Operaciones Git reales y seguras; sin force ni borrado sin alcance; pruebas temporales con conflictos y rutas literales | aceptada |
| T4-UI | Agente de interfaz | Flujos de stash, integración, resolución y confirmaciones | `src/ui/**` | Tipos de dominio y acciones T4 fijados por T4-APP | Política/editor configurables; conflicto visible; confirma abort, stash drop, branch delete y discard | aceptada |
| T4-VERIFY | Agente de verificación | Auditoría independiente de D4 integrada | Sin cambios de producción | T4-APP/T4-GIT/T4-UI integrados | 27 pruebas Windows con Git real; fmt y Clippy sin warnings; revisión de seguridad sin bloqueos | aceptada |

## Entrega 5 — pulido y distribución

| ID | Responsable | Alcance | Archivos | Dependencias | Criterios | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| T5-UI | Agente de interfaz | Jerarquía visual, tema, adaptación a ventana, lectura/teclado, estados y diffs grandes | `src/ui/**` | D4 aceptada; contrato D5 de `CONTRATOS.md`; no altera dominio ni comandos | Interfaz en español coherente; controles legibles con estados ocupados/vacíos/error/truncamiento; conserva todos los flujos; navegación/accesibilidad revisadas en código y manuales solo si el host lo permite | aceptada |
| T5-DIST | Agente de distribución | CI de GitHub Actions, empaquetado Windows/Linux y documentación de instalación | `.github/workflows/**`, `scripts/package-windows.ps1`, `README.md`, `.gitignore` | D4 aceptada; contrato D5 de `CONTRATOS.md`; no publica releases ni usa secretos | ZIP Windows reproducible; matriz CI Windows/Linux y artefactos de ejecución; README documenta Git requerido y límites; no reclama ejecución remota hasta conectar un repo | aceptada |
| T5-VERIFY | Agente de verificación | Auditoría final integrada de MVP y límites de plataforma | Sin cambios de producción | T5-UI/T5-DIST integrados | fmt, Clippy, pruebas y build Windows pasan; workflow revisado; plataforma visual/Linux y mediciones se declaran según evidencia | aceptada |

Entrega 5 aceptada después de verificación Windows y revisión independiente. Linux, GUI manual y ejecución remota de Actions quedan pendientes por entorno/remoto ausentes, documentados en `ESTADO_DEL_PROYECTO.md` y `README.md`.

## Actualización solicitada T6 — acceso a GitHub y pulido de ventana

| ID | Responsable | Alcance | Archivos | Dependencias | Criterios | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| T6-APP | Orquestador | Estado/actions tipados, coordinación de login/verificación, ventana por defecto y documentación | `src/domain.rs`, `src/app.rs`, `src/main.rs`, `ARQUITECTURA_Y_VISION.md`, `CONTRATOS.md`, `TAREAS.md`, `ESTADO_DEL_PROYECTO.md`, `README.md` | Contrato T6 en `CONTRATOS.md`; runner y UI siguen el contrato ya publicado | Verificación explícita bajo demanda; login/verificación se reflejan en snapshot; inicio 1320×860, mínimo 960×640; documentación distingue gh opcional de Git estándar | aceptada |
| T6-GIT | Agente de motor | Lanzamiento visible de `gh auth login` y consulta de estado sin secretos | `src/git/runner.rs` | Tipos exactos T6 en `src/domain.rs` y `CONTRATOS.md` | Usa argumentos separados; Windows/Linux muestran interacción o dan instrucción accionable; no captura token ni cambia configuración de Git silenciosamente; comandos registrados; pruebas del parser/estados sin autenticar cuentas reales | aceptada |
| T6-UI | Agente de interfaz | Botones de acceso/verificación, estados de autenticación y mejoras visuales ligeras | `src/ui/**` | `GitHubAuthStatus`, `UiAction` y contrato T6 ya definidos | Ambos botones invocan actions reales; estado visible en español; mensajes explican consola y verificación; controles responden a busy; conserva flujos D1–D5 | aceptada |
| T6-VERIFY | Agente de verificación | Revisión independiente del flujo T6 integrado | Sin cambios de producción | T6-APP/T6-GIT/T6-UI integrados | Revisión independiente sin defectos confirmados; fmt, Clippy, 28 pruebas y build Windows pasan; Linux, consola interactiva y GUI visual se declaran sin afirmar verificación | aceptada |

La actualización T6 queda cerrada en Windows tras la revisión independiente. La autenticación real, la conexión del remoto del proyecto y la ejecución remota de Actions requieren completar el flujo/URL del repositorio por el usuario.

## Actualización solicitada T7 — ventana compacta, login web y creación de repositorios GitHub

Contrato publicado en `CONTRATOS.md` antes de delegar. El orquestador posee dominio, coordinación, entrypoint y documentación; los agentes no comparten archivos de escritura.

| ID | Responsable | Alcance | Archivos | Dependencias | Criterios | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| T7-APP | Orquestador | Contratos tipados, validación/coordinación, tamaño fijo de ventana y documentos | `src/domain.rs`, `src/app.rs`, `src/main.rs`, docs raíz | Contrato T7 en `CONTRATOS.md`; runner/UI reciben acciones y respuestas fijadas | Ventana 1024×700 no maximizante/fullscreen; app valida slug, login, HEAD/rama y origin antes de solicitar; éxito informa URL y refresca | aceptada |
| T7-GIT | Agente de motor | Login gh en navegador y create+push seguro desde commits | `src/git/runner.rs` | Tipos exactos T7 ya en `src/domain.rs`/`CONTRATOS.md` | Argumentos separados; preflight repetido; no origin overwrite; privado/público; no commit automático ni rollback destructivo; tests sin red real | aceptada |
| T7-UI | Agente de interfaz | Renovación ligera, comandos legibles, login navegador y formulario/confirmación de publicación | `src/ui/**` | `GitHubRepositoryVisibility`, actions y success/error state T7 ya publicados | Funciona en ventana compacta, conserva D1–D6; visibilidad private predeterminada; confirmación explícita y estado disponible según contexto | aceptada |
| T7-VERIFY | Agente verificador | Revisión independiente integrada | Sin cambios de producción | T7-APP/GIT/UI integrados | Revisión sin defectos confirmados restantes; fmt, Clippy, 33 pruebas y build release Windows pasan; GUI visual/Linux/login real/red se declaran no verificados | aceptada |

La actualización T7 queda cerrada en Windows tras revisión independiente. La creación de recursos remotos solo ocurre al usar y confirmar el flujo dentro de Gitzi; no se creó ningún repositorio durante esta entrega. El login, la publicación real, la revisión visual y Linux quedan pendientes de verificarse en el equipo/cuenta del usuario.

## Actualización solicitada T8 — UX, diálogo nativo, terminal integrada y login sin consola

Contrato T8 publicado en `CONTRATOS.md` antes de delegar. El orquestador posee Cargo, dominio, coordinación, entrypoint y documentos; los agentes no comparten archivos de escritura.

| ID | Responsable | Alcance | Archivos | Dependencias | Criterios | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| T8-APP | Orquestador | Panel inferior persistente, contrato compartido, ventana 980×660 y documentación | `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/app.rs`, `src/domain.rs`, documentos raíz | Contrato T8 en `CONTRATOS.md`; API UI definida antes de delegar | Ventana fija; selector de carpetas nativo disponible a UI; consola fija abajo; app sigue sin bloquear egui; docs distinguen navegador externo de terminal y preservan configuración Git global | aceptada |
| T8-UI | Agente de interfaz | Pulido visual/targets de botones, selectores de carpeta y transcript terminal | Solo `src/ui/**` | `rfd = 0.17.2` bloqueado (`xdg-portal`, `wayland`); contrato T8 en `CONTRATOS.md`; API `render_terminal(ui, state, snapshot)` acordada | Selector para abrir/init y destino clone; botón GitHub “Iniciar sesión”; terminal legible y copiable sin pestaña/cards; conserva flujos anteriores, foco y estados | aceptada |
| T8-GIT | Agente de motor | Inicio de sesión web de `gh` sin consola externa | Solo `src/git/**` | Contrato T8 en `CONTRATOS.md`; `CommandStatus::Running` y resultado final acordados por T8-APP | `gh auth login --hostname github.com --web --clipboard`, sin shell ni consola, stdio aislado; no token ni mutación silenciosa de configuración Git; login monitorizado/cancelado al cerrar; pruebas sin red/cuenta | aceptada |
| T8-VERIFY | Agente de verificación | Revisión independiente del flujo integrado T8 | Sin cambios de producción | T8-APP/UI/GIT integrados por el orquestador | Revisar diff/contratos y confirmar cierre del watcher/carreras; fmt, Clippy y pruebas/build Windows pasan; sin login real ni operaciones remotas | aceptada |

La actualización T8 queda cerrada tras verificación integrada Windows y revisión independiente sin bloqueos. La GUI visual, Linux, Actions remotas, autenticación real y publicación contra una cuenta siguen pendientes por entorno/credenciales; no se usó una cuenta ni se añadió un remoto durante esta entrega.
