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
