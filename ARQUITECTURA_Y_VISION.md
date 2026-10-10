# Gitzi: visión y arquitectura

Estado: especificación inicial para guiar la implementación. Este documento describe decisiones de producto y una arquitectura propuesta; no implica que exista una aplicación implementada ni que se hayan medido sus prestaciones.

## 1. Visión

Gitzi será un cliente de escritorio visual para trabajar con Git mediante botones, listas, diferencias e historial. Su prioridad es facilitar el trabajo cotidiano y la colaboración en equipo a personas principiantes e intermedias, incluidas aquellas que conocen Git pero no recuerdan todos sus comandos.

El usuario podrá ver y copiar los comandos realmente utilizados por la aplicación. Ese registro será el único componente didáctico: no habrá tutoriales, explicaciones de comandos, lecciones, glosarios ni recorridos educativos.

La aplicación debe ser ligera, rápida y visualmente cuidada. Se buscará un equilibrio entre una interfaz agradable y el consumo de recursos, evitando tanto una apariencia improvisada como efectos costosos que no mejoren el uso.

Nombre de trabajo: Gitzi, tomado del directorio del proyecto. Puede cambiarse sin afectar la arquitectura.

## 2. Objetivos y límites

### Objetivos

- Cubrir el flujo habitual: abrir o clonar un repositorio, revisar cambios, prepararlos, hacer commit y sincronizar con el remoto.
- Hacer visible la rama actual, su upstream, el estado del árbol de trabajo y los commits pendientes de subir o integrar.
- Facilitar ramas, comparación, historial, stash e integración de cambios de un equipo.
- Mostrar los comandos ejecutados, su resultado y sus errores, sin revelar secretos.
- Mantener la interfaz utilizable mientras Git trabaja.
- Soportar Windows y Linux con la misma lógica de aplicación.
- Crecer por entregas funcionales hasta cubrir operaciones avanzadas de Git.

### Fuera del alcance inicial

- Cubrir todas las opciones posibles de Git.
- Integración con pull requests, issues o APIs de proveedores como GitHub y GitLab.
- Terminal de propósito general, editor de código completo y sistema de plugins.
- Tutoriales, asistentes educativos o explicaciones del significado de cada comando.
- macOS, servicios en la nube propios, cuentas de Gitzi y sincronización de preferencias.
- Consultas remotas periódicas en segundo plano. Fetch será explícito inicialmente.

La visión a largo plazo es amplia, pero cada entrega debe funcionar por sí sola antes de incorporar nuevas funciones.

## 3. Plataformas y tecnología

### Decisiones de base

- Lenguaje: Rust.
- Motor: procesos del ejecutable Git instalado en el sistema.
- Interfaz candidata: egui con eframe. Su adopción definitiva depende de un prototipo que permita comprobar apariencia, accesibilidad, texto y consumo en ambos sistemas.
- Organización inicial: una aplicación Cargo con módulos internos, sin múltiples servicios ni una arquitectura de plugins.
- Persistencia: preferencias y repositorios recientes en un archivo local de configuración, usando rutas apropiadas para cada plataforma.
- Idioma inicial de interfaz: español. Los comandos y la salida original de Git se conservan.

Estas decisiones expresan una dirección de implementación, no una comparación de rendimiento ya demostrada. Antes de fijar dependencias se deben verificar sus versiones, compatibilidad y licencias en documentación oficial.

### Compatibilidad

Windows y Linux se consideran desde el primer día: rutas nativas, argumentos de procesos, codificación, credenciales y empaquetado no deben asumir un único sistema. Si el empaquetado de Linux retrasa el primer lanzamiento, se puede distribuir primero Windows, documentando la validación pendiente de Linux.

Al iniciar, Gitzi debe detectar Git y mostrar un error accionable si no está disponible. No instalará Git automáticamente. La versión mínima soportada se decidirá y documentará según los comandos que use la primera entrega.

## 4. Experiencia de usuario

### Distribución principal

| Zona | Contenido |
| --- | --- |
| Barra superior | Repositorio, rama actual, upstream y acciones Fetch, Pull y Push |
| Panel lateral | Archivos modificados, preparados y sin seguimiento |
| Área central | Diferencias, historial o comparación según la selección |
| Área de commit | Mensaje y acción para confirmar los cambios preparados |
| Panel inferior plegable | Registro de comandos, salida, errores y botón de copiar |

Las acciones frecuentes deben estar a mano. Las avanzadas se agrupan en menús contextuales o secciones desplegables. Los botones deben representar operaciones concretas; se evitará un botón ambiguo de sincronización que encadene decisiones ocultas.

### Criterios visuales

- Tipografía legible, jerarquía clara, espaciado consistente e iconos con etiquetas o nombres accesibles.
- Paleta discreta y contraste suficiente; el estado nunca se comunica únicamente mediante color.
- Tema inicial cuidado, con colores centralizados para permitir otros temas posteriormente.
- Estados vacíos, progreso, errores y acciones deshabilitadas con comportamiento coherente.
- Navegación por teclado y foco visible en los flujos principales.
- Listas e historial con carga progresiva; diferencias grandes con límites explícitos.
- Sin animaciones decorativas continuas ni repintado constante en reposo.

No mostrar paneles vacíos como si una función estuviera implementada. Durante el prototipo pueden usarse datos de ejemplo claramente identificados y aislados de los repositorios reales.

## 5. Alcance funcional por entregas

### Entrega 1: base visual y repositorios

- Crear el proyecto y validar el candidato de interfaz.
- Detectar Git, abrir un repositorio y recordar los recientes.
- Mostrar rama, estado y archivos reales mediante consultas de solo lectura.
- Establecer el ejecutor en segundo plano y el registro de comandos.
- Medir arranque, memoria y CPU en reposo para disponer de una referencia.

Aceptación: se puede abrir un repositorio real sin modificarlo, consultar su estado y ver los comandos utilizados sin bloquear la ventana.

### Entrega 2: cambios y commits

- Inicializar y clonar repositorios.
- Mostrar diferencias entre árbol de trabajo e índice, y entre índice y HEAD.
- Preparar y retirar de preparación archivos completos.
- Crear commits con un mensaje introducido por el usuario.
- Manejar repositorios sin commits y archivos binarios de manera explícita.

Aceptación: el flujo abrir → revisar → preparar → commit funciona y el estado final coincide con Git. Un archivo parcialmente preparado conserva correctamente la separación entre índice y árbol de trabajo.

### Entrega 3: colaboración

- Fetch, pull, push y configuración explícita del upstream cuando falte.
- Crear y cambiar ramas; listar ramas locales y remotas.
- Historial paginado, detalles de commits y comparación entre referencias.
- Mostrar commits adelantados y atrasados respecto al upstream local conocido, junto con el momento del último fetch cuando se conozca.
- Reutilizar la autenticación configurada en Git y manejar su fallo o cancelación.

Aceptación: dos clones de un remoto temporal pueden intercambiar cambios. Un push rechazado muestra el error y nunca se transforma en un push forzado.

### Entrega 4: integración y recuperación

- Stash: crear, listar, inspeccionar, aplicar y eliminar con confirmación cuando corresponda.
- Merge, rebase y revert con acciones explícitas.
- Detectar conflictos, listar archivos afectados y permitir abrirlos en un editor externo configurable.
- Marcar como resuelto mediante preparación del archivo y permitir continuar o abortar la operación cuando Git lo admita.
- Descartar cambios y eliminar ramas con confirmaciones que identifiquen el alcance.

Aceptación: un conflicto no bloquea la aplicación ni provoca reintentos destructivos. Se conserva el estado de la operación y el usuario puede resolverla o abortarla.

### Entrega 5: pulido y distribución

- Revisar accesibilidad, errores, rendimiento y diferencias grandes.
- Probar y empaquetar en Windows y Linux, o declarar con precisión qué plataforma queda pendiente.
- Documentar instalación, dependencia de Git y limitaciones reales.
- Optimizar a partir de mediciones y revisar el tamaño de distribución.

### Ampliaciones posteriores

Tags, cherry-pick, reset, reflog, preparación por fragmentos, grafo completo de ramas, worktrees, submódulos, bisect y soporte específico para Git LFS. No deben presentarse como disponibles antes de implementarse y verificarse.

## 6. Arquitectura lógica

Flujo principal:

```text
Interfaz
   ↓ intención del usuario
Servicios de aplicación
   ↓ operación validada
Constructor de comandos → Ejecutor de Git → Proceso Git
                              ↓ eventos y resultado
                        Analizadores de salida
                              ↓
                     Estado de la aplicación
                              ↓
                          Interfaz

El ejecutor también alimenta el registro de comandos.
La persistencia almacena únicamente preferencias y datos locales de Gitzi.
```

### Responsabilidades

| Módulo | Responsabilidad |
| --- | --- |
| `ui` | Componentes visuales, selección, navegación y presentación del estado |
| `app` | Casos de uso, cola de operaciones, validación y actualización del estado |
| `domain` | Tipos de repositorio, referencias, archivos, diferencias y operaciones |
| `git` | Construcción de argumentos, ejecución, cancelación y análisis de resultados |
| `command_log` | Eventos de ejecución, redacción de secretos y copia de comandos |
| `settings` | Configuración local versionada, recientes y preferencias |
| `platform` | Rutas de configuración, apertura de editor y particularidades del sistema |

Estructura orientativa, que solo se ampliará cuando exista una necesidad real:

```text
src/
  main.rs
  app/
  domain/
  git/
    commands.rs
    runner.rs
    parsers.rs
  ui/
  command_log.rs
  settings.rs
  platform.rs
tests/
```

La interfaz no construye cadenas de shell ni analiza directamente la salida de Git. Solicita operaciones tipadas y recibe estados y eventos. Los módulos de dominio no dependen de la biblioteca gráfica.

## 7. Ejecución de Git

### Procesos y argumentos

- Usar `std::process::Command` o una abstracción equivalente con argumentos separados y directorio de trabajo explícito.
- No ejecutar mediante concatenación de texto en PowerShell, cmd o bash.
- Conservar rutas nativas y manejar espacios, Unicode, saltos de línea y nombres que comiencen por guion.
- Separar opciones y rutas con `--` cuando el comando lo permita; tratar las rutas seleccionadas como rutas literales, no patrones de pathspec.
- Preferir salidas estables y estructuradas, como formatos porcelain y separadores NUL cuando estén disponibles.
- No basar decisiones en frases de salida traducidas ni analizar diffs como fuente del estado del repositorio.
- Mantener la salida original para el usuario y un resultado estructurado para la aplicación.

### Trabajo en segundo plano

Un trabajador y una cola acotada son suficientes inicialmente; no se añadirá un runtime asíncrono sin necesidad comprobada. El hilo de interfaz nunca debe esperar a que termine Git.

Las operaciones que modifican el repositorio se serializan por repositorio. Git conserva sus propios bloqueos: Gitzi no elimina archivos de bloqueo automáticamente. Las consultas se coordinan con las mutaciones para evitar mostrar estados contradictorios.

Cada petición incluye identidad del repositorio y generación del estado. Si el usuario cambia de repositorio, una respuesta tardía no debe sobrescribir la vista actual.

La cancelación debe tener en cuenta el proceso y sus hijos en cada plataforma. Cancelar no equivale a deshacer: después se vuelve a consultar el repositorio y se muestra su estado real.

### Autenticación y configuración existente

Respetar las credenciales, agentes SSH, hooks, firma de commits y configuración de Git del usuario. No almacenar contraseñas ni desactivar hooks o firma para hacer que una operación pase.

Los procesos que necesitan interacción no pueden quedar esperando en una consola oculta indefinidamente. La implementación debe definir un mecanismo compatible o fallar con un mensaje accionable. No se prometerá soporte para todos los mecanismos de autenticación antes de probarlos.

## 8. Registro de comandos

El registro procede del ejecutor real, nunca de una lista de comandos de ejemplo asociada a los botones.

Cada ejecución contiene:

- Identificador, repositorio y directorio de trabajo.
- Origen: acción del usuario o consulta interna.
- Hora de inicio, duración y estado: en cola, ejecutando, terminado, fallido o cancelado.
- Ejecutable y argumentos, representados de forma copiable para un shell definido de cada plataforma.
- Salida estándar, salida de error y código de retorno cuando exista.

El registro muestra todas las ejecuciones, pero agrupa las consultas internas en una sección plegable. Copiar no vuelve a ejecutar nada.

La representación visual del comando es distinta de su ejecución: Gitzi siempre ejecuta argumentos separados. Las comillas y escapes de la representación copiable deben verificarse para el shell elegido. Si se sustituyen secretos por marcadores, el comando copiado refleja esos marcadores.

Redactar credenciales incrustadas en URLs y otros secretos identificables antes de mostrar o persistir datos. No volcar variables de entorno. Usar límites de memoria y de salida para evitar registros ilimitados. El historial de salida será de sesión inicialmente; no se guardará completo en disco por defecto.

## 9. Estado, actualización y rendimiento

- Actualizar después de operaciones propias, al volver al foco y mediante una acción manual.
- Añadir vigilancia de archivos con agrupación de eventos solo si aporta valor comprobado, evitando bucles provocados por las propias consultas.
- No hacer fetch implícito para refrescar la interfaz.
- Identificar que los contadores respecto al remoto se calculan sobre referencias locales conocidas y pueden estar desactualizados.
- Paginar el historial y cargar diferencias bajo demanda.
- Cancelar o ignorar resultados obsoletos y evitar consultas duplicadas.
- Limitar salida capturada y tamaño de diferencias representadas; ofrecer un estado explícito al alcanzar un límite.
- Respetar cambios externos realizados por terminal, editor u otro cliente.

Registrar en cada medición: sistema operativo, hardware, versión de Git, tipo de compilación y tamaño del repositorio. Evaluar arranque en frío, memoria, CPU en reposo, tiempo de consulta de estado, fluidez con operaciones largas y tamaño del paquete.

Los presupuestos numéricos de rendimiento se fijarán después del prototipo. No inventar objetivos cumplidos ni optimizar únicamente el tamaño del ejecutable a costa de la experiencia.

## 10. Integridad y decisiones de Git

- Preparar, hacer commit y hacer push son acciones separadas.
- En Entrega 3, Pull usa avance rápido únicamente y la interfaz muestra esta política; ante divergencia, la operación falla sin iniciar integración. En Entrega 4, persistir la política escogida y ofrecer merge o rebase explícitos, sin cambiar la política global de Git.
- Nunca realizar push forzado automáticamente. Si se añade más adelante, tendrá una acción diferenciada y condiciones explícitas.
- Nunca descartar, resetear, eliminar ramas o borrar stashes automáticamente para superar un error.
- Mostrar confirmaciones concretas para acciones que puedan perder trabajo, con archivos o referencias afectados.
- Detectar HEAD separado, repositorios sin commits, conflictos y operaciones de integración en curso; deshabilitar acciones incompatibles.
- No editar directamente archivos internos de `.git` para cambiar el estado del repositorio.
- No ejecutar comandos sugeridos por archivos, mensajes de commit o contenido del repositorio.
- Probar operaciones mutantes en repositorios temporales, nunca en el repositorio de trabajo del usuario sin que lo solicite.

Estos mensajes y confirmaciones son parte del funcionamiento seguro del producto; no constituyen contenido educativo.

## 11. Verificación

Priorizar pruebas de comportamiento con Git real en repositorios temporales. Cubrir:

- Repositorio vacío, cambios preparados y no preparados, renombrados, eliminados y archivos binarios.
- Rutas con espacios, Unicode y caracteres especiales compatibles con cada sistema.
- Commit y conservación de cambios que no se prepararon.
- Dos clones con un remoto bare local para fetch, push, divergencia y rechazo de push.
- Conflictos, continuación y aborto de integración cuando esas funciones se incorporen.
- Fallos, salida abundante, cancelación y respuestas tardías.
- Registro fiel, escape de comandos copiables y redacción de secretos.

Comprobar manualmente el flujo visual principal, teclado, foco y respuesta de la ventana. Usar formato, análisis estático y pruebas disponibles en el proyecto. No añadir pruebas que solo repitan detalles internos sin comprobar un comportamiento relevante.

Una compilación exitosa en Windows no demuestra compatibilidad con Linux. Informar con precisión qué plataformas y casos se verificaron.

## 12. Forma de implementación con un agente

El documento sirve como contexto estable para un prompt maestro y prompts por entrega. Antes de modificar código, el agente debe inspeccionar las instrucciones y el estado del repositorio, conservar el trabajo existente y elegir la siguiente entrega incompleta.

Cada entrega debe cerrar con:

1. Un flujo funcional real y revisable.
2. Comprobaciones proporcionadas al riesgo del cambio.
3. Un resumen de lo implementado, lo verificado y las limitaciones pendientes.
4. Actualización breve del estado del proyecto para que la siguiente sesión pueda continuar.

No implementar todo de una sola vez, añadir funciones ficticias, introducir dependencias sin motivo o declarar soporte que no se haya verificado. Los cambios sustanciales de arquitectura deben justificarse y reflejarse en este documento.
