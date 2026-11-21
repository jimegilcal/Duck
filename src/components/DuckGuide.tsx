import { Bell, Bot, CalendarDays, FolderOpen, StickyNote } from 'lucide-react';

export function DuckGuide({ onOpenData }: { onOpenData: () => void }) {
  return <>
    <div className="page-heading"><div><h1>Cómo usar Duck</h1><p>Controles, datos y conexiones de esta versión.</p></div></div>
    <section className="settings-section guide-section">
      <h2>Duck en el escritorio</h2>
      <ul>
        <li>Un clic en Duck muestra u oculta el chat. También puedes plegarlo con la × o con Esc.</li>
        <li>Un doble clic abre la app completa. Pulsa ⌘ ⇧ D para ocultar o mostrar a Duck. Para moverlo al monitor que estás usando, elige «Traer a Duck a esta pantalla» en su menú.</li>
        <li>Usa − y + debajo del patito para ajustar su tamaño. Pulsa el número del centro para elegir uno de los siete tamaños. Con clic secundario puedes cambiar los accesorios.</li>
      </ul>
    </section>
    <section className="settings-section guide-section">
      <h2><StickyNote size={16}/> Notas</h2>
      <p>Escribe «anota comprar café» en el chat o pulsa <strong>Nueva nota</strong> en Notas. Encuentra lo guardado en <strong>Hoy</strong>, <strong>Esta semana</strong> o <strong>Historial</strong>. Puedes buscar, editar y eliminar notas desde allí.</p>
      <p>Duck guarda las notas, los recordatorios y el historial que envías en <code>duck.sqlite3</code>, en este Mac. El tamaño y el accesorio se guardan en <code>settings.json</code>. Para hacer una copia, abre la carpeta, sal de Duck y copia toda la carpeta.</p>
      <button className="secondary" onClick={onOpenData}><FolderOpen size={15}/> Abrir carpeta de datos en Finder</button>
    </section>
    <section className="settings-section guide-section">
      <h2><Bell size={16}/> Recordatorios</h2>
      <p>Escribe «recuérdame en 20 minutos hacer una pausa». Duck lo guarda en Recordatorios y te avisa mientras la app esté en marcha, aunque la barra de chat esté oculta.</p>
    </section>
    <section className="settings-section guide-section">
      <h2><CalendarDays size={16}/> Google Calendar y Google Tasks</h2>
      <ol>
        <li>Una vez, añade Google en <strong>Ajustes del Sistema → Cuentas de Internet</strong> y activa <strong>Calendarios</strong>.</li>
        <li>En <strong>Agenda → Elegir calendario</strong>, concede el acceso de macOS y elige el calendario de tu cuenta de Google.</li>
        <li>Escribe «agenda una reunión mañana a las 10:30». Duck guarda el evento en el calendario elegido y te pregunta por los datos que falten.</li>
      </ol>
      <p>Duck recuerda el calendario elegido. macOS se encarga de sincronizar los eventos con Google. Las tareas siguen abriéndose en Google Tasks desde Agenda.</p>
    </section>
    <section className="settings-section guide-section">
      <h2><Bot size={16}/> Claude, ChatGPT, Claude Code y Codex</h2>
      <p>Pulsa Agentes en la línea del chat para ver Codex, Claude Code, ChatGPT y Claude. Duck indica si detecta cada herramienta en este Mac. Las sesiones y conversaciones siguen pendientes de conexión.</p>
    </section>
  </>;
}
