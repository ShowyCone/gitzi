# Estado del proyecto

Entrega actual: 2 — cambios y commits.

Estado verificado al inicio: el directorio contenía únicamente `ARQUITECTURA_Y_VISION.md`; no era un repositorio Git y no había `AGENTS.md` en los directorios del proyecto/ancestros comprobados. La toolchain Rust 1.99.0 estaba instalada en `C:\Users\alexr\.cargo\bin`, pero no estaba en el PATH del proceso. Se inicializó el repositorio local en `main` para conservar los cambios en commits ordenados.

Entrega 1 cerrada: estructura Cargo/eframe, contratos, worker Git de solo lectura, vista de estado/repositorios y coordinación de recientes implementados. Se generó `Cargo.lock` con eframe `0.36.2`. Los contratos y tareas T1 quedaron aceptados tras corregir el comando copiable de Windows, los avisos de Clippy y el formato.

Validación de Entrega 1 en Windows: `cargo fmt --check` pasó; `cargo test` pasó con 9/9 pruebas; `cargo clippy --all-targets -- -D warnings` pasó. `cargo run` compiló e inició `target/debug/gitzi.exe`; el proceso se cerró después de la comprobación. El test del worker inspeccionó un repositorio temporal con rutas Unicode/espacios y cambios staged, unstaged y untracked; usó `--no-optional-locks` y comprobó que `.git/index` permaneciera idéntico. La salida truncada de `status` se rechaza y queda marcada en el registro.

Validación pendiente que no bloquea Entrega 2: no se pudo inspeccionar visualmente la ventana ni hacer la comprobación manual de teclado/foco; la herramienta de Computer Use de esta sesión informa que las APIs de aplicaciones nativas están deshabilitadas. Tampoco se midieron arranque en frío, Working Set o CPU en reposo. Linux no se probó en este host Windows.

Medición pendiente: con un build release, anotar Windows/hardware, versión de Git, tamaño del repo de muestra y toolchain. Tomar 10 tiempos de arranque en frío desde lanzamiento hasta que la ventana responda; para reposo, esperar 60 s tras abrir el mismo repo y registrar Working Set y el incremento de CPU del proceso. No se inventan resultados sin mediciones.

Siguiente paso: implementar y verificar únicamente Entrega 2 según `ARQUITECTURA_Y_VISION.md`. Tras ella, continuar con Entrega 3 en una tarea posterior. La medición del prototipo y la comprobación visual manual siguen en la lista de pendientes.
