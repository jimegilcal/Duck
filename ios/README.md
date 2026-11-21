# Duck para iPhone

Proyecto nativo en [DuckMobile.xcodeproj](DuckMobile.xcodeproj). Incluye un patito 3D dentro de la app, entrada de texto, notas locales y confirmación de eventos. La fecha del evento se elige en la ficha móvil.

Las notas del iPhone se guardan en el propio dispositivo. Para que los eventos aparezcan en Google Calendar, añade la cuenta de Google a Calendario del iPhone y elige esa cuenta en la ficha de Duck. La sincronización de notas con el Mac y los accesorios del escritorio quedan pendientes.

## Instalar para pruebas

1. Abre el proyecto en Xcode con el SDK de iOS.
2. En el objetivo DuckMobile, selecciona tu cuenta en Signing & Capabilities → Team.
3. Elige tu iPhone como destino y pulsa Run. Si el iPhone pide activar el modo desarrollador, sigue los pasos que indica el dispositivo.

El proyecto apunta a iOS 17 o posterior. Para instalarlo en un iPhone personal necesitas firma desde Xcode. Esta entrega contiene el código fuente, sin una app firmada y sin pruebas en un dispositivo iOS. [Apple Developer — firma para pruebas en dispositivos personales](https://developer.apple.com/help/account/basics/about-your-developer-account/).
