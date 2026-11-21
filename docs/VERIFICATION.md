# Verificación de Duck

Entorno de desarrollo: Apple Silicon, macOS 26.3.1, Node 25.2.1, npm 11.6.2 y Rust 1.100.0. Tauri 2.12 permite transparencia sin activar macOSPrivateApi.

## Pruebas automatizadas

- `npm test` comprueba 23 casos: rutas del chat, fechas y horas locales, expresiones cotidianas, aclaraciones y filtros de notas.
- `npm run test:native` comprueba SQLite, persistencia, validación, recordatorios, ciclo de las notificaciones temporales y construcción del borrador de Google Calendar.
- `npm run bundle` genera la app de macOS para Apple Silicon.

## Verificado en macOS

- El paquete optimizado arm64 arranca con el chat plegado. Un clic real sobre el modelo abre el chat y un doble clic real abre el panel.
- El deslizante cambió la mascota de 144 a 103 puntos. Se aplicó el ajuste y se comprobó la reducción en el escritorio. El chat siguió abriéndose a ese tamaño.
- Desde el chat se escribió «¿Puedes agendar una reunión de prueba mañana a las diez y media?». La respuesta abrió Agenda y la revisión mostró «reunión de prueba», 20 de noviembre de 2026 a las 10:30 (Europe/Madrid) y 30 minutos editables. No se guardó un evento en Google como parte de la prueba.
- La compilación del paquete optimizado y la verificación de la firma local en la ruta de salida pasaron.

## Actualización de controles y accesorios

- El paquete actualizado arrancó con la barra plegada. Un clic sobre el modelo abrió el chat. Pulsar Duck de nuevo dejó solo la mascota. Al reabrir, el texto sin enviar se conservó durante la misma ejecución.
- La compilación del frontend y el paquete optimizado arm64 terminaron correctamente. Se verificó la firma local de la app empaquetada.
- En esta pasada no se completó la selección visual de cada accesorio ni la prueba de cada opción del menú secundario. Se pueden comprobar también desde Configuración.

## Comprobaciones manuales

- Arrancar Duck y comprobar que solo se muestra la mascota. Un clic sobre el patito abre el campo y un doble clic abre el panel.
- En Configuración, mover Tamaño de Duck y pulsar Aplicar. Comprobar el tamaño con el chat cerrado y abierto, y volver a abrir la app.
- Escribir una nota desde Duck. Abrir «Ver nota» y comprobar la vista Hoy. Cerrar la app del todo y volver a abrir para verificar persistencia.
- Probar «avísame de llamar a Lucía mañana a las nueve y media» y una frase ambigua. Confirmar que Duck pregunta cuando faltan datos.
- Seleccionar un calendario de Google en Agenda. Escribir «agenda una reunión mañana a las diez», revisar título, hora y duración, pulsar «Guardar evento» y verificar que el evento aparece en Google Calendar.
- Lanzar una notificación de prueba con Duck visible y de nuevo con Duck oculto. Verificar burbuja, ausencia de foco y vuelta a ocultarse.
- Probar transparencia y arrastre junto a otra app. Verificar también posición al reiniciar, varios monitores y consumo con Monitor de Actividad.

Las pruebas automatizadas validan el intervalo y la protección contra confirmaciones duplicadas. Queda por verificar el guardado en una cuenta real de Google y por implementar las conexiones de agentes y conversaciones.

## Controles directos, calendario e iPhone

- La ventana flotante expone dos botones físicos de tamaño. Se pulsaron ambos con el chat plegado y se vio cambiar el tamaño en pantalla, sin usar Configuración. En el paquete final se pasó de 210 a 222 puntos con + y se volvió a 210 con −.
- Desde la mascota se envió un evento de prueba. Se comprobó que Duck pidió confirmación y mostró el título, la fecha, la hora, la duración y el selector de calendario en la misma ventana. No se concedió acceso a un calendario personal durante la prueba y no se creó ningún evento real.
- Se verificó que el paquete de macOS incluye el texto de permiso de Calendario. Las pruebas nativas cubren la validación de hora, el guardado local del borrador y el bloqueo de confirmaciones duplicadas.
- La compilación de macOS configura el patito para todos los Spaces y para las ventanas de otras aplicaciones. Aún queda probar los distintos modos de pantalla completa y Stage Manager con la configuración del usuario.
- El proyecto SwiftUI de iPhone pasó la validación de su archivo de proyecto y la comprobación de tipos usando el SDK instalado de macOS. El SDK de iOS no está instalado. Todavía no se compiló ni se ejecutó en iPhone.

## Recuperación de la mascota y rango ampliado

- El paquete firmado arrancó con la mascota, el tamaño actual entre los botones y la barra de chat plegada. Se observaron cambios de tamaño en la ventana de escritorio.
- Pasaron 23 pruebas de frontend y 13 nativas. La compilación optimizada y la verificación de su firma terminaron correctamente.
- El código nativo vuelve a poner la mascota al frente al cambiar de escritorio o aplicación y ofrece una recuperación manual a la pantalla del cursor.

## Duck sobre Safari

- Se reprodujo la desaparición con la versión anterior al entrar en el espacio de pantalla completa de otra aplicación.
- La versión actual usa un panel flotante de AppKit y arranca como app de barra de menús. Se comprobó que permanecía por delante de una ventana privada de Safari en pantalla completa, que un clic abría el chat y que el campo recibía texto.
- Las pruebas se hicieron con un monitor conectado. Queda por comprobar la configuración con varios monitores y Stage Manager.
