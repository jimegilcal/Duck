type Kind = 'note' | 'reminder' | 'event' | 'task';
type Timing = { day?: string; time?: string; minutes?: number };
type EventNameContext = { eventType?: string; explicitTitle?: boolean };
export type FollowUp =
  | { kind:'unknown'; text:string; missing:'kind' }
  | ({ kind:Kind; title?:string; timing?:Timing; missing:'title'|'day'|'time'|'duration' } & EventNameContext);
export type EventDetails = { title:string; start:string; end:string };

export type Action =
  | {type:'note.create'; body:string}
  | {type:'reminder.create'; title:string; dueAt:string; alarm?:boolean}
  | {type:'calendar.event.create'; event:EventDetails}
  | {type:'calendar.task.propose'; text:string}
  | {type:'report.query' | 'agent.status.query'}
  | {type:'undo.last'; target?:'note'|'reminder'|'event'}
  | {type:'summary.today'}
  | {type:'clarify'; message:string; followUp?:FollowUp};

const notePrefix = /^(?:(?:crea(?:r)?|guarda(?:r|me)?|guárdame|haz(?:me)?|añade|añadir|escribe|escribir|toma(?:r)?)\s+(?:una?\s+)?(?:nota|idea)(?=\s|:|$)|(?:apunta(?:r)?|anota(?:r)?)(?:\s+que)?(?=\s|:|$)|guarda(?:r)?\s+esto(?=\s|:|$)|nota\s*:)/iu;
const reminderPrefix = /^(?:recu[eé]rdame|recordarme|recu[eé]rd[aá]melo|acu[eé]rdame|(?:pon(?:me)?|crea|programa|añade)\s+(?:un\s+)?recordatorio|recordatorio(?=\s|:|$)|av[ií]same|no dejes que se me olvide)(?:\s+que)?(?=\s|:|$)/iu;
const taskPrefix = /^(?:(?:crea(?:r)?|a[ñn][aá]d(?:e(?:me)?|me)|añadir|agrega(?:me|r)?|pon(?:me|er)?|m[eé]te(?:me|r)?|guarda(?:r|me)?|programa(?:me|r)?|anota(?:me|r)?|ap[uú]nta(?:me)?|tengo)\s+)?(?:(?:una?|las|los|estas?|estos?|mis|unas?|unos?|\d+|dos|tres|cuatro|cinco|seis|siete|ocho|nueve|diez)\s+)?(?:tareas?|tare|tara|tasks?)(?=\s|:|$)(?:\s+(?:en|a)\s+(?:google tasks|google calendar|el calendario|mi calendario))?|^(?:(?:pon(?:me)?|a[ñn][aá]d(?:e(?:me)?|me)|anota(?:me)?|agrega(?:me)?)\s+(?:en|a)\s+)?(?:google\s+)?tasks(?=\s|:|$)/iu;
const eventWords = '(?:evento|reuni[oó]?n|cita(?:\\s+m[ée]dica)?|entrevista|videollamada|quedada|m[ée]dico|dentista|fisio|terapia|consulta|peluquer[ií]a|clase|examen|partido|concierto|boda|cumplea[ñn]os|fiesta|festival|presentaci[oó]n|conferencia|charla|seminario|taller|curso|vuelo|viaje|almuerzo|cena|comida|desayuno|brunch|obra|pel[ií]cula|teatro|cine|revisi[oó]n|operaci[oó]n|yoga|pilates|entrega|mudanza|graduaci[oó]n|bautizo|comuni[oó]n)';
const eventPrefix = new RegExp(`^(?:(?:crea(?:r)?|a[ñn][aá]d(?:e(?:me)?|me|ir)|agrega(?:me|r)?|pon(?:me|er)?|m[eé]te(?:me|r)?|agenda(?:me|r)?|ag[eé]ndame|programa(?:me|r)?|ap[uú]nta(?:me)?)\\s+(?:en\\s+(?:google calendar|el calendario|mi calendario)\\s+)?(?:(?:el|la|una?|mi)\\s+)?${eventWords}|(?:pon(?:me)?|m[eé]te(?:me)?|a[ñn][aá]d(?:e(?:me)?|me)|agrega|ap[uú]nta(?:me)?)\\s+en\\s+(?:google calendar|el calendario|mi calendario)|(?:agenda(?:me|r)?|ag[eé]ndame|programa(?:me|r)?)|(?:calendario|evento|reuni[oó]?n|cita)(?=\\s|:|$))(?=\\s|:|$)`,'iu');
const lead = /^(?:duck[,\s]+|hola[,\s]+|oye[,\s]+|por favor[,\s]+|(?:quiero|necesito|me gustar[ií]a)\s+que\s+(?:me\s+)?(?:(?:a[ñn]ad|pong|metan?|cre[ea]|escrib|guard|apunt|anot|program|agend|agregu)[aáeé]s\s+)?|(?:quiero|necesito|me gustar[ií]a)\s+|me puedes\s+|puedes\s+|podr[ií]as\s+)/iu;
const days = ['domingo','lunes','martes','miércoles','jueves','viernes','sábado'];
const numbers: Record<string,number> = {un:1,una:1,uno:1,dos:2,tres:3,cuatro:4,cinco:5,seis:6,siete:7,ocho:8,nueve:9,diez:10,once:11,doce:12,quince:15,veinte:20,treinta:30,cero:0};
const fold = (text:string) => text.normalize('NFD').replace(/\p{Diacritic}/gu,'').toLowerCase();
const trimSeparators = (text:string) => text.replace(/^[\s:;,]+|[\s:;,]+$/gu,'');
function clean(input:string) {
  let text = input.trim().replace(/^[¿¡]+/u,'').replace(/[?!]+$/u,'').replace(/[,\s]+por favor[.!]?$/iu,'').trim();
  while (lead.test(text)) text = text.replace(lead,'').trim().replace(/^[¿¡]+/u,'').trim();
  return text;
}

const months = ['enero','febrero','marzo','abril','mayo','junio','julio','agosto','septiembre','octubre','noviembre','diciembre'];
const dayPattern = /\b(?:(?:para(?:\s+el)?|el|este)\s+)?(pasado\s+mañana|hoy|mañana|lunes|martes|mi[ée]rcoles|jueves|viernes|s[áa]bado|domingo|\d{4}-\d{2}-\d{2}|\d{1,2}\/\d{1,2}(?:\/\d{4})?|\d{1,2}\s+de\s+(?:enero|febrero|marzo|abril|mayo|junio|julio|agosto|septiembre|octubre|noviembre|diciembre)(?:\s+de\s+\d{4})?)\b/giu;
const hourWords = 'un[ao]?|dos|tres|cuatro|cinco|seis|siete|ocho|nueve|diez|once|doce|quince|veinte|treinta';
const clockPattern = new RegExp(`\\ba\\s+(?:las?\\s+|(?:als|lsa|ls)\\s+(?=\\d))(\\d{1,2}|${hourWords})(?::(\\d{2})|(\\s+y\\s+(?:media|cuarto)))?(?:h\\b)?(?:\\s*(a\\.?\\s*m\\.?|p\\.?\\s*m\\.?|de la mañana|de la tarde|de la noche))?(?![\\p{L}\\d]|:\\d)`,'iu');
const pad = (n:number) => String(n).padStart(2,'0');
const dateKey = (date:Date) => [date.getFullYear(), pad(date.getMonth()+1), pad(date.getDate())].join('-');
const timeKey = (date:Date) => pad(date.getHours()) + ':' + pad(date.getMinutes());
function dateFor(day:string, time:string) {
  const date = new Date(day + 'T' + time);
  return Number.isFinite(date.getTime()) && dateKey(date) === day && timeKey(date) === time ? date : null;
}

export function parseSchedule(text:string, now:Date) {
  const relative = /\ben\s+(\d+|media|[\p{L}]+)\s+(minutos?|horas?)\b/iu.exec(text);
  if (relative) {
    const amount = fold(relative[1]);
    const count = amount === 'media' ? .5 : numbers[amount] ?? Number(amount);
    const hours = fold(relative[2]).startsWith('hora');
    const ms = count * (hours ? 3_600_000 : 60_000);
    const valid = Number.isFinite(ms) && ms > 0 && ms <= 365 * 86_400_000 && (amount !== 'media' || hours);
    const date = valid ? new Date(now.getTime()+ms) : null;
    return {
      date, rest:without(text,relative), invalid:!valid, invalidDay:false, invalidTime:!valid,
      day:date ? dateKey(date) : undefined, time:date ? timeKey(date) : undefined, explicitDay:valid,
    };
  }

  const clock = clockPattern.exec(text) ?? /\b(\d{1,2}):(\d{2})\b/u.exec(text);
  const day = Array.from(text.matchAll(dayPattern)).find(match =>
    !clock || match.index < clock.index || match.index >= clock.index + clock[0].length
  );
  let rest = text;
  for (const match of [day,clock].filter((m): m is RegExpExecArray=>m!=null).sort((a,b)=>b.index-a.index)) {
    rest = without(rest, match);
  }

  const date = new Date(now);
  const dayWord = day ? fold(day[1]).replace(/\s+/gu,' ') : '';
  const weekday = days.findIndex(d=>fold(d)===dayWord);
  if (dayWord==='manana' || dayWord==='pasado manana') date.setDate(date.getDate()+(dayWord==='manana'?1:2));
  if (weekday>=0) date.setDate(date.getDate()+((weekday-date.getDay()+7)%7));
  let invalidDay = false;
  if (/^\d/u.test(dayWord)) {
    const [first, second, third] = dayWord.split(/-|\/|\s+de\s+/u);
    const iso = first.length===4;
    const year = Number(iso ? first : third ?? now.getFullYear());
    const month = months.includes(second) ? months.indexOf(second)+1 : Number(second);
    const number = Number(iso ? third : first);
    date.setFullYear(year,month-1,number);
    invalidDay = date.getFullYear()!==year || date.getMonth()!==month-1 || date.getDate()!==number;
  }

  let time: string | undefined;
  if (clock) {
    let hour = numbers[fold(clock[1])] ?? Number(clock[1]);
    const minute = clock[3] ? (fold(clock[3]).includes('media') ? 30 : 15) : Number(clock[2]??0);
    if (clock[4]) {
      if (hour<1 || hour>12) hour=NaN;
      const suffix = fold(clock[4]).replace(/[^a-z]/g,'');
      hour = hour%12 + (suffix==='am'||suffix==='delamanana'?0:12);
    }
    if (Number.isInteger(hour) && hour>=0 && hour<=23 && minute<=59) time=pad(hour)+':'+pad(minute);
  }
  let scheduled = time && !invalidDay ? dateFor(dateKey(date),time) : null;
  if (scheduled && scheduled<=now && (!day||weekday>=0)) {
    date.setDate(date.getDate()+(weekday>=0?7:1));
    scheduled = dateFor(dateKey(date),time!);
  }
  const invalidTime = !!clock && (!time || (!invalidDay && !scheduled));
  return {date:scheduled,rest,invalid:invalidDay||invalidTime,invalidDay,invalidTime,
    day:day && !invalidDay ? dateKey(date) : undefined,time:invalidTime ? undefined : time,explicitDay:!!day};
}
function without(text:string,match:RegExpExecArray) {
  return text.slice(0,match.index) + ' '.repeat(match[0].length) + text.slice(match.index+match[0].length);
}
function content(text:string) {
  return trimSeparators(text).replace(/^para\s+/iu,'').replace(/\s{2,}/gu,' ').trim();
}
const kindLabel: Record<Kind,string> = { note:'la nota', reminder:'el recordatorio', event:'el evento', task:'la tarea' };
const cancelText = /^(?:cancela(?:r|lo)?|olvida(?:r|lo)|olvídalo|déjalo|mejor no)$/iu;
const kindAnswer = /^(?:(?:como|es)\s+)?(?:una?\s+)?(nota|recordatorio|evento|cita|reuni[oó]?n|tarea)(?:\s*[:;,]\s*(.+))?$/iu;
type Pending = Extract<FollowUp,{kind:Kind}>;

function ask(kind:Kind, missing:Pending['missing'], title?:string, timing?:Timing, eventName?:EventNameContext):Action {
  const subject = title ? ' «'+title+'»' : ' '+kindLabel[kind];
  const message = {
    title: kind==='note' ? '¿Qué quieres guardar en la nota?' : kind==='task' ? '¿Qué tarea quieres guardar?' : kind==='reminder' ? '¿Qué quieres que te recuerde?' : '¿De qué es o con quién es? Así le pongo un título concreto.',
    day: '¿Qué día quieres para'+subject+'?',
    time: '¿A qué hora quieres'+subject+'?',
    duration: '¿Cuánto debe durar'+subject+'? Dime la duración en minutos.',
  }[missing];
  return {type:'clarify',message,followUp:{kind,missing,title,timing,...eventName}};
}

function durationFrom(text:string, answering:boolean) {
  const pattern = /\b(?:durante\s+|duraci[oó]n\s*:?\s*|de\s+)(\d+|media|[\p{L}]+)\s+(minutos?|horas?)\b/iu;
  const match = pattern.exec(text) ?? (answering ? /^(\d+|media|[\p{L}]+)(?:\s+(minutos?|horas?))?$/iu.exec(text) : null);
  if (!match) return { minutes:undefined, rest:text, invalid:false };
  const count = fold(match[1])==='media' ? .5 : numbers[fold(match[1])] ?? Number(match[1]);
  const minutes = count * (match[2]?.toLowerCase().startsWith('hora') ? 60 : 1);
  const valid = Number.isInteger(minutes) && minutes>=1 && minutes<=1440;
  return {minutes:valid ? minutes : undefined, rest:without(text,match), invalid:!valid};
}

function eventTitle(text:string) {
  const title = content(text)
    .replace(/^(?:con\s+(?:el\s+)?t[ií]tulo|(?:que\s+)?se\s+llama?|ll[aá]ma(?:lo|la|le|se)?|tit[uú]la(?:lo|la|le)?)\s*:?\s*/iu,'')
    .replace(/^(?:tengo|hay)\s+(?:una?\s+)?/iu,'')
    .replace(/^(?:he\s+quedado|qued[oéeó]|quedamos)\s+/iu,'')
    .replace(/^(?:me|nos)\s+(?:veo|vemos|re[uú]no|reunimos)\s+/iu,'')
    .replace(/^(?:voy|vamos)\s+/iu,'')
    .replace(/^(?:pon(?:me|er)?|poner|m[eé]te(?:me|r)?|a[ñn]ad(?:e(?:me)?|ir)|agrega(?:me|r)?|crea(?:r)?|programa(?:me|r)?|ap[uú]nta(?:me)?|anota(?:me|r)?)\s+/iu,'')
    .replace(/^a(?:l\b|\s+la\b)\s*/iu,'')
    .replace(/^(?:el|la|los|las)\s+/iu,'')
    .replace(/\s*\ba\s+(?:las?|als|lsa|ls)\s*[.!?]*$/iu,'')
    .replace(/^una?\s+/iu,'')
    .replace(/^reuni[oó]?n\b/iu,'reunión');
  return title.replace(/^\p{L}/u,first=>first.toLocaleUpperCase('es'));
}

const genericEvent = /^(?:evento|reuni[oó]?n|cita|entrevista|videollamada|llamada|quedada|encuentro|actividad|consulta)[.!?]*$/iu;
const explicitEventName = /^(?:ll[aá]malo|tit[uú]lalo|con\s+(?:el\s+)?t[ií]tulo|que\s+se\s+llame|llamad[oa]|titulad[oa])(?=\s|:|$)/iu;

function complete(kind:Kind, body:string, now:Date, pending?:Pending):Action {
  if (kind==='note') {
    const title = body.trim();
    return title ? {type:'note.create',body:title} : ask(kind,'title');
  }
  if (kind==='task') {
    const schedule = parseSchedule(body,now);
    const title = content(schedule.rest);
    if (!title || /^(?:algo|una cosa|la tarea|tarea)[.!?]*$/iu.test(title)) {
      return ask(kind,'title',undefined,{day:schedule.day??pending?.timing?.day,time:schedule.time??pending?.timing?.time});
    }
    let text = body.trim();
    if (pending?.timing) {
      const retainedDay = schedule.explicitDay ? undefined : pending.timing.day;
      const retainedTime = schedule.time ? undefined : pending.timing.time;
      const when = [retainedDay,retainedTime && 'a las '+retainedTime].filter(Boolean).join(' ');
      if (when) text += ' · '+when;
    }
    return {type:'calendar.task.propose',text};
  }

  const explicitTitle = kind==='event' ? /(?:^\s*|\b(?:con\s+(?:el\s+)?t[ií]tulo|que\s+se\s+llame|llamad[oa]|titulad[oa]|ll[aá]malo|tit[uú]lalo)\s*:?\s*)[«“"]([^»”"\n]+)[»”"]/iu.exec(body) : null;
  const details = explicitTitle ? without(body,explicitTitle) : body;
  const duration = kind==='event' ? durationFrom(details,pending?.missing==='duration') : {rest:details,minutes:undefined,invalid:false};
  const range = kind==='event' && /\bde\s+(?:las?\s+)?(\d{1,2}(?::\d{2})?)\s+a\s+(?:las?\s+)?(\d{1,2}(?::\d{2})?)(?![:\d])(?:\s+(de la mañana|de la tarde|de la noche|a\.?m\.?|p\.?m\.?))?/iu.exec(duration.rest);
  const rangePeriod = range ? range[3] ?? '' : '';
  if (range) duration.rest = duration.rest.slice(0,range.index)+' a las '+range[1]+' '+rangePeriod+' '+duration.rest.slice(range.index+range[0].length);
  const hourWord = fold(duration.rest.trim().split(/\s+/u)[0]);
  const bareHour = pending?.missing==='time' && (/^\d/u.test(hourWord) || Object.hasOwn(numbers,hourWord)) && /^(?:\d{1,2}|[\p{L}]+)(?:\s+y\s+(?:media|cuarto))?(?:\s+(?:a\.?m\.?|p\.?m\.?|de la (?:mañana|tarde|noche)))?$/iu.test(duration.rest);
  const schedule = parseSchedule(bareHour ? 'a las '+duration.rest : duration.rest,now);
  const subjectText = trimSeparators(schedule.rest);
  const recoveredTitle = content(subjectText).replace(/^(?:de|que|llamad[oa]|titulad[oa])\s+/iu,'');
  let title = pending?.title ?? recoveredTitle;
  let eventName: EventNameContext | undefined;
  if (kind==='event') {
    const namedOnPurpose = pending?.title ? pending.explicitTitle : !!explicitTitle || explicitEventName.test(subjectText);
    if (!pending?.title) {
      const topic = subjectText.replace(/^es\s+(?=para|con|de|sobre)\b/iu,'');
      const addType = pending?.eventType && !/^evento$/iu.test(pending.eventType) && /^(?:con|de|para|sobre)\s+/iu.test(topic);
      title = explicitTitle?.[1].trim() || eventTitle(addType ? pending.eventType+' '+topic : recoveredTitle);
    }
    if (!namedOnPurpose && genericEvent.test(title)) {
      eventName = {eventType:title.replace(/[.!?]+$/u,'')};
      title = '';
    } else if (namedOnPurpose) {
      eventName = {explicitTitle:true};
    } else if (!title && pending?.eventType) {
      eventName = {eventType:pending.eventType};
    }
  }
  if (kind==='reminder' && /^(?:algo|una cosa|el recordatorio|recordatorio)[.!?]*$/iu.test(title)) title = '';
  const timing:Timing = {...pending?.timing};
  if (schedule.explicitDay) timing.day = schedule.day;
  if (schedule.invalidTime || schedule.time) timing.time = schedule.time;
  if (duration.invalid) timing.minutes = 0;
  if (duration.minutes) timing.minutes = duration.minutes;
  if (range) {
    const end = parseSchedule('a las '+range[2]+' '+rangePeriod,now).time;
    const minutesOfDay = (time:string) => {
      const [hour,minute] = time.split(':').map(Number);
      return 60*hour+minute;
    };
    const interval = schedule.time && end ? minutesOfDay(end)-minutesOfDay(schedule.time) : 0;
    timing.minutes = interval>0 && (!duration.minutes || duration.minutes===interval) ? interval : 0;
  }
  if (!title) return ask(kind,'title',undefined,timing,eventName);
  if (!timing.day) return ask(kind,'day',title,timing,eventName);
  if (!timing.time) return ask(kind,'time',title,timing,eventName);
  const date = dateFor(timing.day,timing.time);
  if (timing.day < dateKey(now)) return ask(kind,'day',title,{...timing,day:undefined},eventName);
  if (!date || date<=now) return ask(kind,'time',title,{...timing,time:undefined},eventName);
  if (kind==='reminder') return {type:'reminder.create',title,dueAt:date.toISOString()};
  if (timing.minutes===0) return ask(kind,'duration',title,timing,eventName);
  const end = new Date(date.getTime()+(timing.minutes??30)*60_000);
  return {type:'calendar.event.create',event:{title,start:date.toISOString(),end:end.toISOString()}};
}

function command(text:string): {kind:Kind; body:string; explicit:boolean} | undefined {
  for (const [kind,prefix] of [['task',taskPrefix],['reminder',reminderPrefix],['event',eventPrefix],['note',notePrefix]] as const) {
    const matched = prefix.exec(text);
    if (!matched) continue;
    const named = kind==='event' ? /\b(reuni[oó]?n|cita(?:\s+m[ée]dica)?|entrevista|videollamada|quedada|m[ée]dico|dentista|fisio|terapia|consulta|peluquer[ií]a|clase|examen|partido|concierto|boda|cumplea[ñn]os|fiesta|festival|presentaci[oó]n|conferencia|charla|seminario|taller|curso|vuelo|viaje|almuerzo|cena|comida|desayuno|brunch|obra|pel[ií]cula|teatro|cine|revisi[oó]n|operaci[oó]n|yoga|pilates|entrega|mudanza|graduaci[oó]n|bautizo|comuni[oó]n)$/iu.exec(matched[0])?.[1] : undefined;
    const remainder = text.slice(matched[0].length);
    let body = trimSeparators(remainder);
    if (named) body = [named,body].filter(Boolean).join(' ');
    const bareEvent = /^(?:calendario|evento|reuni[oó]?n|cita)$/iu.test(matched[0]) && !/^\s*:/u.test(remainder);
    return {kind,body,explicit:kind!=='event'||!bareEvent};
  }
  if (/^(?:tengo|hay)\s+(?:una?\s+)?(?:cita(?:\s+m[ée]dica)?|reuni[oó]?n|entrevista|videollamada|quedada|m[ée]dico|dentista|fisio|terapia|consulta|peluquer[ií]a|clase|examen|partido|concierto|boda|cumplea[ñn]os|fiesta|festival|presentaci[oó]n|conferencia|charla|seminario|taller|curso|vuelo|viaje|almuerzo|cena|comida|desayuno|brunch|obra|pel[ií]cula|teatro|cine|revisi[oó]n|operaci[oó]n|yoga|pilates|entrega|mudanza|graduaci[oó]n|bautizo|comuni[oó]n)(?=\s|$)/iu.test(text)) {
    return {kind:'event',body:text.replace(/^(?:tengo|hay)\s+(?:una?\s+)?/iu,''),explicit:true};
  }
  if (/^(?:he\s+quedado|qued[oéeó]|quedamos)\s+con\b/iu.test(text)) {
    return {kind:'event',body:text,explicit:true};
  }
  if (/^(?:me|nos)\s+(?:veo|vemos|re[uú]no|reunimos)\s+con\b/iu.test(text)) {
    return {kind:'event',body:text,explicit:true};
  }
  if (/^(?:voy|vamos)\s+a(?:l\s+|\s+la\s+)/iu.test(text)) {
    return {kind:'event',body:text,explicit:true};
  }
  if (/^(?:ideas? para|ideas?:|mejora para|apunte:)/iu.test(text)) return {kind:'note',body:text,explicit:false};
  if (/^(?:tengo que|no olvides|que no se me olvide)\s+/iu.test(text)) {
    return {kind:'reminder',body:text.replace(/^(?:tengo que|no olvides|que no se me olvide)\s+/iu,''),explicit:true};
  }
}

type ContextMessage = {role:string;body:string;action:string|null;createdAt?:string;followUp?:FollowUp|null};

function inferAction(text:string, original:string, now:Date):Action {
  const schedule = parseSchedule(text,now);
  const hasSchedule = schedule.explicitDay || !!schedule.time || schedule.invalidTime;
  const subject = content(schedule.rest);
  const appointment = /^(?:(?:tengo|hay)\s+)?(?:una?\s+)?(?:reuni[oó]?n|cita(?:\s+m[ée]dica)?|entrevista|videollamada|quedada)(?=\s|[,.:]|$)/iu;
  const scheduledActivity = /^(?:(?:tengo|hay)\s+)?(?:una?\s+)?(?:comida con|cena con|caf[eé] con|cerveza con|copas con|he quedado|dentista|consulta|examen|clase|vuelo|m[ée]dico|fisio|terapia|peluquer[ií]a|ginec[oó]logo|oculista|nutricionista|boda|cumplea[ñn]os|fiesta|festival|concierto|partido|almuerzo|desayuno|brunch|merienda|comida|cena|obra|pel[ií]cula|cine|teatro|presentaci[oó]n|conferencia|charla|seminario|taller|curso|formaci[oó]n|entrega|firma|notario|gestor[ií]a|tr[aá]mite|mudanza|viaje|yoga|pilates|gimnasio|nataci[oó]n|p[aá]del|tenis|revisi[oó]n|operaci[oó]n|an[aá]lisis|radiograf[ií]a|ecograf[ií]a|graduaci[oó]n|bautizo|comuni[oó]n)(?=\s|[,.:]|$)/iu;
  const eventVerb = /^(?:he\s+quedado|qued[oéeó]|quedamos)\s+con\b|^(?:me|nos)\s+(?:veo|vemos|re[uú]no|reunimos)\s+con\b|^(?:voy|vamos)\s+a(?:l\s+|\s+la\s+)/iu;
  if (appointment.test(subject) || eventVerb.test(subject) || (hasSchedule && scheduledActivity.test(subject))) {
    const details = text.replace(/\b(?:tengo|hay)\s+(?:una?\s+)?/iu,'');
    return complete('event',details,now);
  }
  const obligation = /^(?:tengo que|debo|hay que|acu[eé]rdate de|acu[eé]rdame(?:\s+de)?|que no se me pase)\s+/iu;
  const action = /^(?:llamar|comprar|escribir|enviar|mandar|entregar|pagar|recoger|llevar|revisar|preparar|reservar|renovar|responder|tomar|recuerda|avisar)(?=\s|[,.:]|$)/iu;
  if (obligation.test(subject) || (hasSchedule && (!subject || action.test(subject)))) {
    return complete('reminder',text.replace(obligation,''),now);
  }
  if (!text || /^(?:hola|buenas|buenos d[ií]as|buenas tardes|ayuda)[.!]*$/iu.test(text)) {
    return {type:'clarify',message:'Dime lo que tienes en mente y me encargo de guardarlo o programarlo.'};
  }
  if (/^(?:gracias|muchas gracias|perfecto|vale|ok|de acuerdo)[.!]*$/iu.test(text)) {
    return {type:'clarify',message:'De nada.'};
  }
  const looksLikeNote = /\b(?:idea|apunte|nota|pensamiento|reflexi[oó]n|concepto)\b/iu.test(subject);
  const actionVerb = /^(?:pon(?:me|er)?|m[eé]te(?:me|r)?|a[ñn]ad(?:e(?:me)?|ir)|agrega(?:me|r)?|programa(?:me|r)?|ap[uú]nta(?:me)?|anota(?:me|r)?)/iu.test(text);
  if (hasSchedule && subject && !schedule.invalidTime && !looksLikeNote && (actionVerb || /\bde\s+\d{1,2}(?::\d{2})?\s+a\s+\d{1,2}(?::\d{2})?\b/iu.test(text))) {
    return complete('event',text,now);
  }
  return {type:'note.create',body:original.trim()};
}

export function interpret(input:string, now=new Date()):Action {
  return interpretWithContext(input,[],now);
}

// «Alarma»: a reminder that rings. It is read as a normal reminder and then marked as an alarm.
const alarmLead = /^(?:(?:pon(?:me)?|crea|programa|a[ñn]ade|configura|activa)\s+(?:una?\s+)?alarma|alarma|despi[eé]rtame)(?=\s|:|$)/iu;

// A clarifying question about an alarm keeps being about an alarm until it is answered or cancelled.
function alarmContinues(messages:ContextMessage[]) {
  for (let index=messages.length-1; index>=0; index--) {
    const message = messages[index];
    if (message.role==='assistant') {
      if (message.action!=='clarify' || !message.followUp) return false;
    } else if (cancelText.test(clean(message.body))) return false;
    else if (alarmLead.test(clean(message.body))) return true;
  }
  return false;
}

export function interpretWithContext(input:string,messages:ContextMessage[],now=new Date()):Action {
  const text = clean(input);
  const lead = alarmLead.exec(text);
  const alarm = !!lead || (messages.at(-1)?.role==='assistant' && !command(text) && alarmContinues(messages));
  if (!alarm) return interpretCore(input,messages,now);
  let result:Action;
  if (lead) {
    const rest = text.slice(lead[0].length);
    result = interpretCore('recuérdame'+rest,messages,now);
    if (result.type==='clarify' && result.followUp?.kind==='reminder' && result.followUp.missing==='title') {
      result = interpretCore(`recuérdame${rest} ${/^despi/iu.test(lead[0]) ? 'Despertar' : 'Alarma'}`,messages,now);
    }
  } else result = interpretCore(input,messages,now);
  return result.type==='reminder.create' ? {...result,alarm:true} : result;
}

function interpretCore(input:string,messages:ContextMessage[],now:Date):Action {
  const text = clean(input);
  const last = messages.at(-1);
  let pending = last?.followUp;
  if (!pending && last?.action==='clarify' && /^¿(?:Cuándo|Qué) quieres que te (?:lo )?recuerde/iu.test(last.body)) {
    const previous = messages.at(-2);
    const request = previous?.role==='user' ? command(clean(previous.body)) : undefined;
    if (request?.kind==='reminder') {
      const original = complete('reminder',request.body,previous?.createdAt ? new Date(previous.createdAt) : now);
      if (original.type==='clarify') pending = original.followUp;
    }
  }
  if (cancelText.test(text)) return {type:'clarify',message:'De acuerdo, lo dejo aquí.'};
  if (/^(?:(?:cu[aá]l es el )?estado de (?:los )?(?:agentes|codex|claude(?: code)?)|agentes)$/iu.test(text)) return {type:'agent.status.query'};
  if (/^(?:reporting|resumen|mi resumen|c[oó]mo voy|qu[eé] tengo pendiente[s]?|dame un resumen|mu[eé]strame (?:el|mi) resumen)$/iu.test(text)) return {type:'report.query'};

  if (/^(?:qu[eé]\s+tengo\s+hoy|qu[eé]\s+hay\s+hoy|mi\s+d[ií]a|mi\s+agenda(?:\s+de\s+hoy)?|agenda\s+de\s+hoy|lo\s+de\s+hoy|(?:mu[eé]strame|ens[eé][ñn]ame|dame)\s+(?:lo\s+de\s+hoy|mi\s+d[ií]a|mi\s+agenda))$/iu.test(text)) return {type:'summary.today'};

  const undoAll = /^(?:b[oó]rr[aá](?:lo|r(?:lo)?)?|el[ií]m[ií]n[aá](?:lo|r(?:lo)?)?|qu[ií]ta(?:lo|r(?:lo)?)?|deshaz(?:lo)?|b[oó]rra\s+(?:eso|esto)|quita\s+(?:eso|esto)|el[ií]mina\s+(?:eso|esto))$/iu;
  const undoNote = /^(?:b[oó]rra|elimina|quita)\s+(?:la\s+)?(?:[uú]ltima\s+)?nota$/iu;
  const undoReminder = /^(?:b[oó]rra|elimina|quita)\s+(?:el\s+)?(?:[uú]ltimo?\s+)?recordatorio$/iu;
  const undoEvent = /^(?:b[oó]rra|elimina|quita)\s+(?:el\s+)?(?:[uú]ltimo?\s+)?evento$/iu;
  if (undoNote.test(text)) return {type:'undo.last',target:'note'};
  if (undoReminder.test(text)) return {type:'undo.last',target:'reminder'};
  if (undoEvent.test(text)) return {type:'undo.last',target:'event'};
  if (undoAll.test(text)) return {type:'undo.last'};

  const request = command(text);
  if (pending?.kind==='unknown') {
    const choice = kindAnswer.exec(text);
    if (choice) {
      const word = fold(choice[1]);
      const kind = word==='nota' ? 'note' : word==='tarea' ? 'task' : word==='recordatorio' ? 'reminder' : 'event';
      return complete(kind,choice[2] || pending.text,now);
    }
  }
  if (request?.explicit) {
    const continuing = pending?.kind===request.kind && pending.missing==='title' ? pending : undefined;
    return complete(request.kind,request.body,now,continuing);
  }
  const inferred = request ? complete(request.kind,request.body,now) : inferAction(text,input,now);
  if (!pending || pending.kind==='unknown') return inferred;
  if (pending.missing==='title') return complete(pending.kind,text,now,pending);

  const inferredQuestion = inferred.type==='clarify' ? inferred.followUp : undefined;
  const hasNewSubject = inferred.type==='reminder.create' || inferred.type==='calendar.event.create' ||
    (inferredQuestion && inferredQuestion.kind!=='unknown' && !!inferredQuestion.title);
  if (hasNewSubject) return inferred;

  // A date in a new sentence must not accidentally complete the previous appointment.
  const schedule = parseSchedule(text,now);
  if (inferred.type==='note.create' && (schedule.explicitDay || schedule.time) && content(schedule.rest)) return inferred;
  return complete(pending.kind,text,now,pending);
}

export function inPeriod(value:string,period:'today'|'week'|'all',now=new Date()):boolean {
  if(period==='all')return true;
  const start=new Date(now);
  start.setHours(0,0,0,0);
  if(period==='week')start.setDate(start.getDate()-((start.getDay()+6)%7));
  const end=new Date(start);
  end.setDate(end.getDate()+(period==='week'?7:1));
  const date=new Date(value);
  return date>=start && date<end;
}
