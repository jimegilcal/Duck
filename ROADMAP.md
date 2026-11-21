# Próximos pasos

## 1. Conversación como entrada principal

Mantener la interacción solicitada: personaje y campo debajo en una sola composición de escritorio, respuestas compactas y detalle en la app. Ampliar el intérprete a fechas más naturales, varias acciones, correcciones y referencias al historial. Conectar después un modelo de lenguaje mediante un adaptador intercambiable; explicar qué se envía, permitir elegir proveedor y confirmar las acciones externas. Mantener el modo local y la validación de acciones en Rust. No ejecutar código producido por el modelo.

## 2. Google Calendar y Google Tasks

OAuth para aplicación de escritorio con scopes mínimos, tokens en Keychain y posibilidad de desconectar. Transformar las propuestas de texto en una revisión con título, fecha, zona horaria, duración y calendario/lista destino. Escribir solo tras la confirmación, guardar el ID remoto y evitar duplicados en reintentos. Importar resultados reales al reporting.

Los eventos requieren inicio y fin. Google Tasks es una API distinta: actualmente su campo `due` conserva el día, pero descarta la hora. No prometer una hora exacta de tarea mediante ese campo; usar un recordatorio local o evento cuando corresponda. Ver [documentación de Tasks](https://developers.google.com/workspace/tasks/reference/rest/v1/tasks).

## 3. Notificaciones y rendimiento

Verificar más escenarios con Spaces, monitores externos, pantalla completa, suspensión y Concentración. Considerar NSPanel y un hit-test nativo más fino si la aproximación actual no resulta suficiente. Añadir outbox con estados de intento/entrega, cola visual de avisos y reintentos limitados. Medir CPU, memoria, energía y gráficos durante una sesión larga en M2; reducir frecuencia cuando no haya interacción.

## 4. Voz

Permiso de micrófono solicitado al activar la función. Grabación local, indicador claro, cancelación y revisión de transcripción antes de ejecutar acciones. Transcripción por adaptador; no enviar audio sin decisión explícita del usuario.

## 5. Conversaciones y agentes

- Claude Code: investigar hooks oficiales y eventos de sesiones/proyectos.
- Codex: investigar herramientas y eventos soportados; estados de trabajo, fin, error y aprobación.
- ChatGPT: estudiar una integración oficial disponible, sin leer otras conversaciones automáticamente.

El usuario quiere tanto conversaciones de Claude/ChatGPT como sesiones de Claude Code/Codex. Investigar por separado las APIs y los mecanismos de compartir/importar conversaciones disponibles. No suponer que iniciar sesión en una app concede acceso a todas sus conversaciones ni reutilizar cookies/tokens del navegador. La vinculación real requiere un flujo soportado y consentimiento del usuario.

Comenzar por observación de eventos autorizados. No implementar aprobaciones remotas sin una interfaz oficial verificable y una confirmación concreta de cada operación. Nunca simular sesiones o actividad.

## 6. Reporting y proyectos

Proyectos, tendencias semanales, carga prevista, tareas completadas y eventos externos reales. Separar actividad local, pruebas y proveedores conectados. Enlaces desde cada métrica al registro que la sustenta. No convertir simples contadores en métricas de productividad inventadas.

## 7. Actividad y distribución

Seguimiento de actividad de aplicaciones/proyectos solo tras consentimiento granular y elección de fuentes. Retención configurable, exportación y eliminación de datos. Accesibilidad/pantalla únicamente si una funcionalidad concreta lo necesita y el usuario lo habilita.

Firma Developer ID, notarización, icono final, actualizaciones firmadas, accesibilidad del chat con VoiceOver, navegación por teclado y localización. Modelo GLB original y catálogo de skins, sin modificar la arquitectura del asistente.
