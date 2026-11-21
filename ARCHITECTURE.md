# Arquitectura de Duck

## Dos ventanas, una sola aplicación

1. `duck`: transparente, sin decoración, siempre encima. El personaje es Three.js real y lleva un campo compacto debajo para escribir. Al enviar, aparece una respuesta breve entre el personaje y el campo; el historial se conserva en SQLite. Al plegar la entrada queda solo la mascota y su ventana deja de recibir foco de teclado. Al desplegar explícitamente el campo se permite el foco para escribir. Duck arranca con el campo plegado. El modelo usa el tamaño configurado de 64 a 480 puntos al abrir el chat. El chat mide al menos 300 puntos de ancho y se amplía con los tamaños grandes. La altura adicional depende de si hay una respuesta.
2. `panel`: aplicación ampliable para consultar y editar notas/recordatorios, ver reporting y configurar Duck. Se abre únicamente por una acción del usuario.

Rust es propietario de datos y ciclo de vida. Los WebViews son clientes. Cerrar el panel lo oculta. Un clic alterna el campo de la mascota y un doble clic abre el panel. El texto sin enviar se conserva en memoria al plegar el campo. Tray y atajo siguen funcionando. El plugin single-instance evita dos planificadores. Al abrir el chat se intenta conservar el anclaje de la mascota. Si falta espacio, la ventana se desplaza dentro del monitor y vuelve al anclaje al cerrarse.

## Flujo de instrucciones

`ChatWindow → api.submit → interpretWithContext → submit_chat (Rust) → SQLite → respuesta y enlace de detalle`.

Acciones tipadas: `note.create`, `reminder.create`, `calendar.event.create`, `calendar.task.propose`, `agent.status.query`, `report.query`, `undo.last`, `summary.today`, `clarify`.

El reconocimiento es determinista, en español y sin LLM. Solo mantiene contexto para aclarar la fecha de un recordatorio. El backend valida los datos de nuevo. Cada mensaje lleva un UUID de petición; mensaje, acción y respuesta se guardan en una única transacción. Reintentar el mismo UUID no repite la acción. No hay evaluación de código ni ejecución de comandos.

## Carpetas

- `src/components/DuckScene.tsx`: modelo procedural, materiales, accesorios y control de render.
- `src/components/PetWindow.tsx`: gestos, eventos, animaciones de entrada/salida y burbuja.
- `src/components/ChatWindow.tsx`: campo compacto debajo de Duck, respuesta contextual y enlaces a detalles.
- `src/components/Panel.tsx`: notas, recordatorios, reporting, propuestas, agentes y preferencias.
- `src/core/interpreter.ts`: clasificación y aclaraciones locales, testeadas con Vitest.
- `src/core/api.ts`: frontera IPC tipada para la interfaz.
- `src/core/adapters.ts`: contratos para futuras fuentes de eventos, calendario y voz.
- `src-tauri/src/appearance.rs`: menú contextual de macOS para el tamaño, los accesorios y el chat.
- `src-tauri/src/desktop.rs`: ventanas, bandeja/visibilidad, atajo, posición, preferencias y máscara de interacción.
- `src-tauri/src/lifecycle.rs`: reglas de vencimiento de avisos temporales, con pruebas.
- `src-tauri/src/storage.rs`: esquema y operaciones SQLite, validación y adquisición transaccional de recordatorios vencidos.
- `src-tauri/src/chat.rs`: transacciones del chat, historial y propuestas locales de Google.
- `src-tauri/src/notifications.rs`: entrega centralizada y planificador independiente del WebView.

## SQLite y preferencias

SQLite está embebido en el binario mediante rusqlite; no se instala un servidor. Una conexión protegida por mutex, WAL, timeout y consultas parametrizadas. El esquema usa `user_version=2`, creado de forma idempotente. Las pruebas abren una base temporal, cierran y vuelven a abrirla.

Las preferencias de escritorio se almacenan aparte en JSON, con valores por defecto y reemplazo mediante renombrado. La posición usa coordenadas físicas; el tamaño, puntos lógicos. Se comprueba que la posición inicial esté en un monitor actual y, si no, se recupera en el principal.

## Notificaciones

El temporizador nativo revisa los vencimientos cada dos segundos. Una transacción marca el recordatorio avisado y guarda un evento con ID estable, evitando duplicados en la siguiente revisión y tras reiniciar.

Cada `DuckEvent` contiene ID, proveedor, sesión/proyecto opcionales, estado, tipo, título, descripción, timestamp y metadatos opcionales. Las pruebas usan `provider=test` y `notification.test`, nunca un proveedor de agente real.

El evento se registra antes de intentar la entrega. macOS recibe una notificación por su plugin oficial. Duck puede mostrar además una burbuja sin activar la app. La visibilidad deseada se mantiene separada del plazo temporal: al vencer ocho segundos se conserva Duck si estaba visible, o se oculta tras la salida. Un nuevo aviso prolonga el plazo; mostrarlo manualmente conserva la nueva preferencia. Los avisos simultáneos quedan en el historial; la burbuja muestra el más reciente.

Limitación: el registro persistido prueba que el evento se generó, no que macOS mostró un banner ni que el usuario lo vio. Si el proceso cae entre el commit y la entrega, se conserva el evento pero no se reintenta automáticamente. Un outbox con estados de entrega es una mejora futura.

## Transparencia y foco

Tauri >=2.12 permite transparencia en macOS sin activar `macOSPrivateApi`. La ventana de la mascota parte de `focus=false` y `focusable=false`; solo habilita foco mientras el campo de escritura está abierto. Los avisos usan `show()`, nunca `set_focus()`. Solo una acción explícita del usuario enfoca chat o panel. AppKit permite a la mascota aparecer en todos los Spaces. Cuando cambia la app activa, el Space o una pantalla, el coordinador nativo vuelve a ordenar al frente la mascota mientras su visibilidad esté habilitada, sin hacerla ventana clave. El atajo y el menú de rescate la colocan junto a la esquina de la pantalla del cursor. El comportamiento en pantalla completa depende de macOS.

CSS `pointer-events:none` no basta para que los clics atraviesen un WebView. Duck renderiza una máscara alpha de 64×64 una vez al preparar/cambiar el tamaño de la escena. Rust consulta la posición global del cursor cada 32 ms y alterna `set_ignore_cursor_events` según la silueta, los botones de tamaño, el campo de chat o la burbuja. El acceso al cursor no observa otras ventanas ni requiere accesibilidad. Margen de dos celdas para animación y arrastre. Hay una latencia de hasta unos 32 ms y la máscara aproxima la silueta animada; no es precisión por píxel.

## Rendimiento y evolución visual

Sin texturas remotas ni modelo externo. Geometrías de baja complejidad, sin sombras dinámicas ni posprocesado, resolución limitada. Three.js usa render bajo demanda y un temporizador de 24 FPS solo cuando la mascota está activa y la animación está habilitada. El modelo ilustrativo del panel se renderiza estático.

`DuckModel` puede sustituirse por un componente GLTF sin alterar chat, persistencia, ventana ni eventos. `Skin` separa identidad del personaje y accesorios.

## Integraciones futuras

Claude Code y Codex llegan a Duck mediante *hooks* locales: `scripts/duck_agent_hook.py` escribe un archivo pequeño por sesión en `/tmp/duck-agent-events-<uid>/` y `src-tauri/src/agents.rs` los lee. No hay integración con ChatGPT ni con Claude Desktop. `EventSource` y `CalendarAdapter` son contratos del lado de la interfaz, no servicios independientes. Google Calendar y Google Tasks son APIs diferentes; las propuestas permanecen locales con `pending_connection` hasta revisar los detalles. Duck guarda desde el chat cuando conoce el título, el día y la hora. Si falta un dato conserva la solicitud entre mensajes. EventKit escribe en el calendario de internet seleccionado, y macOS sincroniza esa cuenta. Si no puede guardar, deja una propuesta con los datos completos en Agenda. Duck solicita permiso al listar calendarios y no lee los eventos existentes. El estado `saving` bloquea confirmaciones duplicadas. Un fallo recuperable devuelve el borrador a `pending_connection`. Si la app se interrumpe durante el guardado, se debe comprobar el calendario antes de reintentarlo. Google Tasks sigue usando el navegador y OAuth directo queda para más adelante.

El objetivo SwiftUI en `ios/` tiene su propio modelo 3D, notas del iPhone y confirmación con EventKit. Usa un almacén local independiente.

Fuentes técnicas: [Tauri window configuration](https://v2.tauri.app/reference/config/), [notificaciones](https://docs.rs/tauri-plugin-notification/latest/tauri_plugin_notification/struct.Notification.html), [atajos](https://v2.tauri.app/plugin/global-shortcut/), [Google Calendar](https://developers.google.com/workspace/calendar/api/guides/create-events), [Google Tasks](https://developers.google.com/workspace/tasks/reference/rest/v1/tasks).
