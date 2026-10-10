# Tareas: Entrega 1

| ID | Responsable | Alcance | Archivos | Dependencias | Criterios | Estado |
| --- | --- | --- | --- | --- | --- | --- |
| T1-APP | Orquestador | Manifiesto, dominio, coordinación, configuración local y conexión de módulos | `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/domain.rs`, `src/app.rs`, `src/settings.rs`, docs raíz | Arquitectura leída; contratos en `CONTRATOS.md` | Compila, inicia, detecta Git en segundo plano, coordina acciones, persiste recientes y no escribe en el repo abierto | aceptada |
| T1-GIT | Agente de motor | Worker Git, consulta de detección/estado y registro real | `src/git/**` | Contrato fijado en `CONTRATOS.md`; recibe tipos ya definidos | Argumentos separados, cwd explícito, `--no-optional-locks`, consulta porcelain de solo lectura, salida acotada con truncamiento señalado/rechazado, respuesta no bloqueante; pruebas del repo temporal e índice sin cambios | aceptada |
| T1-UI | Agente de interfaz | Vista operativa de Entrega 1 | `src/ui/**` | Contrato fijado en `CONTRATOS.md`; snapshot y acciones ya definidos | Español; apertura por ruta, recientes, rama/upstream/estado, carga/error, registro plegable y copiar; refresco deshabilitado sin repo; sin lógica de proceso ni botones falsos | aceptada |
| T1-VERIFY | Agente de verificación | Revisión independiente del flujo integrado y criterios | Sin cambios de producción | Integración coherente de T1-APP/T1-GIT/T1-UI | Inspección, compilación, formato, Clippy y 9 pruebas en Windows; limitaciones visuales y de rendimiento documentadas | aceptada |
