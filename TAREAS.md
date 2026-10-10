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
