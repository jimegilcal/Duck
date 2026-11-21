import { describe,it,expect } from 'vitest';
import { interpret,inPeriod,interpretWithContext } from './interpreter';
const now=new Date(2026,10,17,10,0);
describe('intérprete local',()=>{
 it('crea notas y preserva texto',()=>expect(interpret('nota: Café y reunión')).toEqual({type:'note.create',body:'Café y reunión'}));
 it('calcula intervalos reales',()=>expect(interpret('recuérdame en 10 minutos: llamar',now)).toEqual({type:'reminder.create',title:'llamar',dueAt:new Date(now.getTime()+600000).toISOString()}));
 it('interpreta mañana en hora local',()=>expect(interpret('recuérdame mañana a las 09:30: correo',now)).toEqual({type:'reminder.create',title:'correo',dueAt:new Date(2026,10,18,9,30).toISOString()}));
 it.each(['recuérdame hoy a las 09:00: tarde','recuérdame mañana a las 25:00: hora','recuérdame 2027-02-30 a las 09:00: fecha','recuérdame en 0 minutos: ahora','nota:'])('pide aclaración: %s',s=>expect(interpret(s,now).type).toBe('clarify'));
 it('pide el tema si solo recibe el tipo de evento',()=>expect(interpret('evento: reunión')).toMatchObject({type:'clarify',followUp:{kind:'event',missing:'title'}}));
 it('reconoce consulta de agentes',()=>expect(interpret('estado de codex').type).toBe('agent.status.query'));
});
describe('vistas por fecha local',()=>{
 it('incluye hoy y excluye mañana',()=>{expect(inPeriod(new Date(2026,10,17,0,1).toISOString(),'today',now)).toBe(true);expect(inPeriod(new Date(2026,10,18).toISOString(),'today',now)).toBe(false)});
 it('la semana empieza el lunes',()=>{expect(inPeriod(new Date(2026,10,16).toISOString(),'week',now)).toBe(true);expect(inPeriod(new Date(2026,10,15,23,59).toISOString(),'week',now)).toBe(false)});
});

describe('chat y clasificación',()=>{
 it('entiende una nota sin comando formal',()=>expect(interpret('Apunta una idea brillante')).toEqual({type:'note.create',body:'una idea brillante'}));
 it('reconoce eventos y tareas por separado',()=>{expect(interpret('Crea un evento reunión de equipo mañana')).toMatchObject({type:'clarify',followUp:{kind:'event',missing:'time'}});expect(interpret('tarea: preparar informe').type).toBe('calendar.task.propose')});
 it('reconoce reporting',()=>expect(interpret('Dame un resumen').type).toBe('report.query'));
 it('acepta hora al final',()=>expect(interpret('recuérdame llamar a Ana mañana a las 09:30',now)).toEqual({type:'reminder.create',title:'llamar a Ana',dueAt:new Date(2026,10,18,9,30).toISOString()}));
 it('pide la hora que falta y entiende la respuesta',()=>{const original='recuérdame llamar a Ana';const a=interpret(original,now);expect(a.type).toBe('clarify');if(a.type!=='clarify')throw Error('aclaración esperada');const context=[{role:'user',body:original,action:null},{role:'assistant',body:a.message,action:'clarify',followUp:a.followUp}];expect(interpretWithContext('en 10 minutos',context,now)).toEqual({type:'reminder.create',title:'llamar a Ana',dueAt:new Date(now.getTime()+600000).toISOString()});});
});

describe('peticiones en lenguaje cotidiano', () => {
  it('entiende peticiones corteses y conserva acentos', () => {
    expect(interpret('Oye Duck, ¿puedes guardar una nota: Café con Lucía?')).toEqual({type:'note.create',body:'Café con Lucía'});
  });

  it('entiende fechas y horas sin exigir dos puntos', () => {
    expect(interpret('Avísame de llamar a Lucía mañana a las nueve y media', now)).toEqual({
      type:'reminder.create',title:'llamar a Lucía',dueAt:new Date(2026,10,18,9,30).toISOString()
    });
    expect(interpret('Ponme un recordatorio para hacer una pausa en media hora', now)).toEqual({
      type:'reminder.create',title:'hacer una pausa',dueAt:new Date(now.getTime()+30*60_000).toISOString()
    });
  });

  it('conserva el día al responder solo con la hora', () => {
    const body = 'Recuérdame llamar el jueves';
    const clarification = interpret(body, now);
    if (clarification.type!=='clarify') throw Error('Se esperaba una aclaración');
    expect(interpretWithContext('a las 3 de la tarde', [
      {role:'user',body,action:null},
      {role:'assistant',body:clarification.message,action:'clarify',followUp:clarification.followUp}
    ], now)).toEqual({type:'reminder.create',title:'llamar',dueAt:new Date(2026,10,19,15).toISOString()});
  });

  it('distingue una tarea de una cita', () => {
    expect(interpret('Crea una tarea en Google Calendar: enviar presupuesto')).toEqual({type:'calendar.task.propose',text:'enviar presupuesto'});
    expect(interpret('Agéndame una cita con Laura el viernes a las 10',now)).toEqual({type:'calendar.event.create',event:{title:'Cita con Laura',start:new Date(2026,10,20,10).toISOString(),end:new Date(2026,10,20,10,30).toISOString()}});
  });
});

describe('preguntas que conservan los datos',()=>{
  function reply(input:string,previous:ReturnType<typeof interpret>) {
    if(previous.type!=='clarify')throw Error('Se esperaba una pregunta');
    return interpretWithContext(input,[{role:'assistant',body:previous.message,action:'clarify',followUp:previous.followUp}],now);
  }

  it('crea un evento después de tres respuestas cortas',()=>{
    let action=interpret('Ponme un evento',now);
    expect(action).toMatchObject({followUp:{missing:'title'}});
    action=reply('Café con Ana',action);
    expect(action).toMatchObject({followUp:{missing:'day',title:'Café con Ana'}});
    action=reply('el 25 de noviembre',action);
    expect(action).toMatchObject({followUp:{missing:'time',timing:{day:'2026-11-25'}}});
    expect(reply('15:30',action)).toEqual({
      type:'calendar.event.create',event:{title:'Café con Ana',start:new Date(2026,10,25,15,30).toISOString(),end:new Date(2026,10,25,16).toISOString()}
    });
  });

  it('guarda texto ambiguo como nota sin pedir una categoría',()=>{
    const text = 'Un nombre para el proyecto. ¿Luna?';
    expect(interpret(text,now)).toEqual({type:'note.create',body:text});
  });

  it('reconoce una cita y un recordatorio cuando la fecha va primero',()=>{
    expect(interpret('Mañana a las 11 tengo una entrevista con Laura',now)).toMatchObject({
      type:'calendar.event.create',event:{title:'Entrevista con Laura',start:new Date(2026,10,18,11).toISOString()}
    });
    expect(interpret('El jueves llamar a Ana',now)).toMatchObject({
      type:'clarify',followUp:{kind:'reminder',title:'llamar a Ana',missing:'time',timing:{day:'2026-11-19'}}
    });
  });

  it('reconoce una tarea mencionando Google Tasks',()=>{
    expect(interpret('Pon en Google Tasks enviar informe',now)).toEqual({type:'calendar.task.propose',text:'enviar informe'});
  });

  it('conserva la fecha al preguntar qué hay que hacer',()=>{
    const task = interpret('Ponme una tarea para mañana',now);
    expect(task).toMatchObject({followUp:{kind:'task',missing:'title',timing:{day:'2026-11-18'}}});
    expect(reply('Enviar el presupuesto',task)).toEqual({type:'calendar.task.propose',text:'Enviar el presupuesto · 2026-11-18'});

    const reminder = interpret('Acuérdame una cosa el jueves a las 15',now);
    expect(reminder).toMatchObject({followUp:{kind:'reminder',missing:'title'}});
    expect(reply('Llamar a Ana',reminder)).toEqual({
      type:'reminder.create',title:'Llamar a Ana',dueAt:new Date(2026,10,19,15).toISOString()
    });
  });

  it('un recordatorio nuevo no completa un evento que espera fecha',()=>{
    const pendingEvent = interpret('Entrevista con Laura a las 11',now);
    expect(pendingEvent).toMatchObject({followUp:{kind:'event',missing:'day'}});
    expect(reply('El jueves llamar a Ana',pendingEvent)).toMatchObject({
      type:'clarify',followUp:{kind:'reminder',title:'llamar a Ana',missing:'time'}
    });
    expect(reply('Mañana tengo una idea para el logo',pendingEvent)).toEqual({
      type:'note.create',body:'Mañana tengo una idea para el logo'
    });
    expect(reply('el jueves',pendingEvent)).toMatchObject({
      type:'calendar.event.create',event:{title:'Entrevista con Laura',start:new Date(2026,10,19,11).toISOString()}
    });
  });

  it('no asume el día de una hora suelta',()=>{
    const action=interpret('Recuérdame llamar a Ana a las 15',now);
    expect(action).toMatchObject({followUp:{missing:'day',timing:{time:'15:00'}}});
    expect(reply('mañana',action)).toMatchObject({type:'reminder.create',dueAt:new Date(2026,10,18,15).toISOString()});
  });

  it('usa la duración explícita',()=>{
    expect(interpret('Cita con Laura mañana a las 10 durante 45 minutos',now)).toMatchObject({
      type:'calendar.event.create',event:{end:new Date(2026,10,18,10,45).toISOString()}
    });
  });

  it('reconoce la hora aunque las letras de «las» estén cambiadas de orden',()=>{
    const pendingMeeting = interpret('ponme una reunin hoy a als 13',now);
    expect(pendingMeeting).toMatchObject({
      type:'clarify',followUp:{missing:'title',eventType:'Reunión',timing:{day:'2026-11-17',time:'13:00'}}
    });
    expect(reply('con Ana',pendingMeeting)).toEqual({
      type:'calendar.event.create',event:{title:'Reunión con Ana',start:new Date(2026,10,17,13).toISOString(),end:new Date(2026,10,17,13,30).toISOString()}
    });
    expect(interpret('ponme una reunion de equipo mañana a als 13:15',now)).toMatchObject({
      type:'calendar.event.create',event:{title:'Reunión de equipo',start:new Date(2026,10,18,13,15).toISOString()}
    });
  });

  it('separa el título de una hora incompleta y respeta un título entre comillas',()=>{
    expect(interpret('ponme una reunión de equipo mañana a las',now)).toMatchObject({
      type:'clarify',followUp:{missing:'time',title:'Reunión de equipo'}
    });
    expect(interpret('ponme un evento para mañana a las 13',now)).toMatchObject({
      type:'clarify',followUp:{missing:'title',timing:{day:'2026-11-18',time:'13:00'}}
    });
    expect(interpret('ponme un evento llamado «Proyecto 13:00» mañana a las 15h',now)).toMatchObject({
      type:'calendar.event.create',event:{title:'Proyecto 13:00',start:new Date(2026,10,18,15).toISOString()}
    });
  });
});

describe('detección de eventos cotidianos', () => {
  // Eventos con día y hora → se crean directamente
  it.each([
    ['cena con Marta el viernes a las 21', 'Cena con Marta', new Date(2026,10,20,21).toISOString()],
    ['mañana a las 10 reunión con el equipo', 'Reunión con el equipo', new Date(2026,10,18,10).toISOString()],
    ['reunión de equipo hoy a las 17:00', 'Reunión de equipo', new Date(2026,10,17,17).toISOString()],
    ['examen de inglés el lunes a las 9', 'Examen de inglés', new Date(2026,10,23,9).toISOString()],
    ['clase de yoga mañana a las 19', 'Clase de yoga', new Date(2026,10,18,19).toISOString()],
    ['cita médica el viernes a las 16:00', 'Cita médica', new Date(2026,10,20,16).toISOString()],
    ['concierto el 12 de diciembre a las 20:30', 'Concierto', new Date(2026,11,12,20,30).toISOString()],
    ['partido de fútbol el domingo a las 12', 'Partido de fútbol', new Date(2026,10,22,12).toISOString()],
    ['el lunes tengo una presentación a las 11', 'Presentación', new Date(2026,10,23,11).toISOString()],
  ])('evento completo: %s', (frase, title, start) => {
    const a = interpret(frase, now);
    expect(a).toMatchObject({type:'calendar.event.create', event:{title, start}});
  });

  // Eventos con día pero sin hora → preguntan la hora
  it.each([
    ['tengo médico el jueves', 'Médico', '2026-11-19'],
    ['cumpleaños de Pablo el 3 de diciembre', 'Cumpleaños de Pablo', '2026-12-03'],
    ['el sábado voy a la boda de Laura', 'Boda de Laura', '2026-11-21'],
    ['almuerzo con Carlos mañana', 'Almuerzo con Carlos', '2026-11-18'],
    ['fiesta el sábado', 'Fiesta', '2026-11-21'],
    ['viaje a Madrid el 5 de diciembre', 'Viaje a Madrid', '2026-12-05'],
    ['entrega del proyecto el 15 de diciembre', 'Entrega del proyecto', '2026-12-15'],
    ['he quedado con Sara el martes', 'Con Sara', '2026-11-17'],
  ])('pregunta la hora: %s', (frase, title, day) => {
    const a = interpret(frase, now);
    expect(a).toMatchObject({type:'clarify', followUp:{kind:'event', title, missing:'time', timing:{day}}});
  });

  // Eventos sin día ni hora → preguntan el día
  it.each([
    ['tengo dentista', 'Dentista'],
    ['tengo médico', 'Médico'],
    ['tengo boda', 'Boda'],
    ['quedo con Juan', 'Con Juan'],
  ])('pregunta el día: %s', (frase, title) => {
    expect(interpret(frase, now)).toMatchObject({type:'clarify', followUp:{kind:'event', title, missing:'day'}});
  });

  // Verbos que implican evento
  it('quedo con Juan mañana a las 6', () => {
    expect(interpret('quedo con Juan mañana a las 6', now)).toMatchObject({
      type:'calendar.event.create', event:{title:'Con Juan', start:new Date(2026,10,18,6).toISOString()}
    });
  });
  it('me veo con Ana el lunes', () => {
    expect(interpret('me veo con Ana el lunes', now)).toMatchObject({
      type:'clarify', followUp:{kind:'event', title:'Con Ana', missing:'time'}
    });
  });
  it('voy al médico mañana', () => {
    expect(interpret('voy al médico mañana', now)).toMatchObject({
      type:'clarify', followUp:{kind:'event', title:'Médico', missing:'time'}
    });
  });
  it('voy a la boda de Laura el sábado', () => {
    expect(interpret('voy a la boda de Laura el sábado', now)).toMatchObject({
      type:'clarify', followUp:{kind:'event', title:'Boda de Laura', missing:'time'}
    });
  });

  // «apúntame en el calendario» detecta evento
  it('apúntame en el calendario el médico mañana', () => {
    expect(interpret('apúntame en el calendario el médico mañana', now)).toMatchObject({
      type:'clarify', followUp:{kind:'event', missing:'time'}
    });
  });
  it('méteme en el calendario cena con Ana el viernes a las 21', () => {
    expect(interpret('méteme en el calendario cena con Ana el viernes a las 21', now)).toMatchObject({
      type:'calendar.event.create', event:{title: expect.stringContaining('Ana')}
    });
  });

  // «tengo que» sigue siendo recordatorio, no evento
  it('tengo que ir al médico sigue siendo recordatorio', () => {
    expect(interpret('tengo que ir al médico el jueves', now)).toMatchObject({
      type:'clarify', followUp:{kind:'reminder', missing:'time'}
    });
  });

  // Las notas no se convierten en evento
  it.each([
    'Un nombre para el proyecto. ¿Luna?',
    'Mañana tengo una idea para el logo',
    'apunta que la alarma del coche falla',
  ])('sigue siendo nota: %s', frase => {
    expect(interpret(frase, now).type).toBe('note.create');
  });
});

describe('alarmas',()=>{
  const at=(y:number,mo:number,d:number,h:number,mi:number)=>new Date(y,mo,d,h,mi).toISOString();
  it.each([
    ['pon una alarma mañana a las 7 para despertarme','despertarme',at(2026,10,18,7,0)],
    ['alarma: llamar al médico mañana a las 9','llamar al médico',at(2026,10,18,9,0)],
    ['crea una alarma el viernes a las 8 para la reunión','la reunión',at(2026,10,20,8,0)],
    ['alarma mañana a las 7','Alarma',at(2026,10,18,7,0)],
    ['despiértame mañana a las 6:45','Despertar',at(2026,10,18,6,45)],
  ])('crea una alarma: %s',(frase,title,dueAt)=>expect(interpret(frase,now)).toEqual({type:'reminder.create',title,dueAt,alarm:true}));
  it('calcula una alarma en minutos',()=>
    expect(interpret('pon una alarma en 15 minutos para el horno',now)).toEqual({type:'reminder.create',title:'el horno',dueAt:new Date(now.getTime()+900000).toISOString(),alarm:true}));
  it('un recordatorio normal no es alarma',()=>{
    const action=interpret('recuérdame llamar mañana a las 9',now);
    expect(action).toMatchObject({type:'reminder.create'});
    expect(action).not.toHaveProperty('alarm');
  });
  it.each(['guarda una nota: probar la alarma del móvil','apunta que la alarma del coche falla'])('una nota que nombra la alarma sigue siendo nota: %s',s=>expect(interpret(s,now).type).toBe('note.create'));
  it('una hora suelta pregunta el día, como un recordatorio',()=>expect(interpret('ponme una alarma a las 18:30: sacar el pan',now)).toMatchObject({type:'clarify',followUp:{kind:'reminder',missing:'day'}}));
  it('no crea una alarma en el pasado',()=>expect(interpret('pon una alarma hoy a las 8',now).type).toBe('clarify'));

  const ask=(first:string,...replies:string[])=>{
    const messages:any[]=[];let action:any=interpret(first,now);
    messages.push({role:'user',body:first,action:null});
    for(const reply of replies){
      messages.push({role:'assistant',body:action.message??'',action:action.type==='clarify'?'clarify':'reminders',followUp:action.followUp});
      action=interpretWithContext(reply,messages,now);
      messages.push({role:'user',body:reply,action:null});
    }
    return {action,messages};
  };
  it('sigue siendo alarma cuando hay que contestar la hora',()=>
    expect(ask('pon una alarma para despertarme','mañana a las 7').action).toEqual({type:'reminder.create',title:'despertarme',dueAt:at(2026,10,18,7,0),alarm:true}));
  it('sigue siendo alarma tras varias preguntas',()=>
    expect(ask('pon una alarma mañana','a las 7:30').action).toMatchObject({type:'reminder.create',title:'Alarma',alarm:true}));
  it('un recordatorio nuevo no hereda una alarma sin terminar',()=>{
    const {messages}=ask('pon una alarma para despertarme');
    const first:any=interpret('pon una alarma para despertarme',now);
    messages.push({role:'assistant',body:first.message,action:'clarify',followUp:first.followUp});
    const action=interpretWithContext('recuérdame llamar a Ana mañana a las 9',messages,now);
    expect(action).toMatchObject({type:'reminder.create',title:'llamar a Ana'});
    expect(action).not.toHaveProperty('alarm');
  });
  it('tras cancelar una alarma, lo siguiente no es alarma',()=>{
    const first:any=interpret('pon una alarma para despertarme',now);
    const messages:any[]=[
      {role:'user',body:'pon una alarma para despertarme',action:null},
      {role:'assistant',body:first.message,action:'clarify',followUp:first.followUp},
      {role:'user',body:'cancela',action:null},
      {role:'assistant',body:'De acuerdo, lo dejo aquí.',action:'clarify'},
    ];
    expect(interpretWithContext('mañana a las 9 llamar a Ana',messages,now)).not.toHaveProperty('alarm');
  });
});

describe('ponme/méteme/apúntame + palabra de evento (eventPrefix ampliado)',()=>{
  const now=new Date('2026-11-17T10:00:00');

  it('ponme una entrevista → evento, pregunta día',()=>{
    const r:any=interpret('ponme una entrevista',now);
    expect(r.type).toBe('clarify');
    expect(r.followUp).toBeDefined();
  });

  it('ponme una entrevista el lunes a las 10 → evento (pregunta título por ser genérico)',()=>{
    const r:any=interpret('ponme una entrevista el lunes a las 10',now);
    expect(r.type).toBe('clarify');
    expect(r.type).not.toBe('note.create');
  });

  it('ponme un concierto el sábado → evento, pregunta hora',()=>{
    const r:any=interpret('ponme un concierto el sábado',now);
    expect(r.type).toBe('clarify');
    expect(r.followUp?.title).toBe('Concierto');
  });

  it('ponme una cena con Ana el viernes a las 21 → evento directo',()=>{
    const r:any=interpret('ponme una cena con Ana el viernes a las 21',now);
    expect(r.type).toBe('calendar.event.create');
    expect(r.event?.title).toBe('Cena con Ana');
  });

  it('méteme un médico el jueves a las 12 → evento directo',()=>{
    const r:any=interpret('méteme un médico el jueves a las 12',now);
    expect(r.type).toBe('calendar.event.create');
    expect(r.event?.title).toBe('Médico');
  });

  it('apúntame una clase de yoga mañana a las 9 → evento directo',()=>{
    const r:any=interpret('apúntame una clase de yoga mañana a las 9',now);
    expect(r.type).toBe('calendar.event.create');
    expect(r.event?.title).toBe('Clase de yoga');
  });

  it('ponme una boda el sábado → evento, pregunta hora',()=>{
    const r:any=interpret('ponme una boda el sábado',now);
    expect(r.type).toBe('clarify');
    expect(r.followUp?.title).toBe('Boda');
  });

  it('programa una consulta el martes a las 16 → evento, pregunta título (consulta es genérica)',()=>{
    const r:any=interpret('programa una consulta el martes a las 16',now);
    expect(r.type).toBe('clarify');
    expect(r.type).not.toBe('note.create');
  });

  it('ponme un vuelo el domingo a las 7 → evento directo',()=>{
    const r:any=interpret('ponme un vuelo el domingo a las 7',now);
    expect(r.type).toBe('calendar.event.create');
    expect(r.event?.title).toBe('Vuelo');
  });
});

describe('borrar y resumen del día',()=>{
  const now=new Date('2026-11-17T10:00:00');

  it('bórralo → undo.last sin target',()=>{
    expect(interpret('bórralo',now).type).toBe('undo.last');
    expect(interpret('borra eso',now).type).toBe('undo.last');
    expect(interpret('elimínalo',now).type).toBe('undo.last');
    expect(interpret('quítalo',now).type).toBe('undo.last');
  });

  it('borra la nota → undo.last target note',()=>{
    const r:any=interpret('borra la nota',now);
    expect(r.type).toBe('undo.last');
    expect(r.target).toBe('note');
  });

  it('borra el recordatorio → undo.last target reminder',()=>{
    const r:any=interpret('borra el recordatorio',now);
    expect(r.type).toBe('undo.last');
    expect(r.target).toBe('reminder');
  });

  it('borra el evento → undo.last target event',()=>{
    const r:any=interpret('borra el evento',now);
    expect(r.type).toBe('undo.last');
    expect(r.target).toBe('event');
  });

  it('qué tengo hoy → summary.today',()=>{
    expect(interpret('qué tengo hoy',now).type).toBe('summary.today');
    expect(interpret('mi agenda',now).type).toBe('summary.today');
    expect(interpret('lo de hoy',now).type).toBe('summary.today');
    expect(interpret('agenda de hoy',now).type).toBe('summary.today');
  });

  it('borrar un recordatorio no es nota',()=>{
    expect(interpret('borra el último recordatorio',now).type).toBe('undo.last');
  });

  it('qué hay hoy no es nota',()=>{
    expect(interpret('qué hay hoy',now).type).toBe('summary.today');
  });
});
