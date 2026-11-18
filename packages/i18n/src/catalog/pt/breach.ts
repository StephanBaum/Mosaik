/**
 * Brazilian Portuguese UI strings: breach. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "Verificação de vazamentos",
  "breach.card.off":
    "Desativada. O Mosaic nunca contata o Have I Been Pwned, a menos que você ative isto.",
  "breach.card.about":
    "Compara suas senhas com vazamentos de dados conhecidos. Só um trecho de 5 caracteres de um código embaralhado por senha sai deste dispositivo.",
  "breach.mode.label": "Verificar em vazamentos de dados conhecidos",
  "breach.mode.off": "Desativada",
  "breach.mode.manual": "Somente quando eu escolher",
  "breach.mode.weekly": "Toda semana, após desbloquear",
  "breach.card.checkNow": "Verificar agora",
  "breach.card.stop": "Parar verificação",
  "breach.card.turnOff": "Desativar",
  "breach.progress.starting": "Verificando suas senhas…",
  "breach.progress.step": "Verificando… {done} de {total}",
  "breach.result.never": "Ainda não verificado.",
  "breach.result.last":
    "Última verificação em {date, date, medium}. {found, plural, =0 {Nenhuma senha foi encontrada em um vazamento.} one {# senha foi encontrada em um vazamento.} other {# senhas foram encontradas em vazamentos.}}",
  "breach.result.finished":
    "{checked, plural, one {# senha verificada} other {# senhas verificadas}}. {found, plural, =0 {Nenhuma foi encontrada em um vazamento.} one {# foi encontrada em um vazamento.} other {# foram encontradas em vazamentos.}}",
  "breach.result.upToDate": "Tudo foi verificado recentemente. Nada novo para consultar.",
  "breach.error.offline":
    "Não foi possível acessar o Have I Been Pwned. Os últimos resultados são exibidos. Tente de novo quando estiver online.",
  "breach.error.rateLimited":
    "O Have I Been Pwned pediu que o Mosaic diminuísse o ritmo. Tente de novo em alguns minutos.",
  "breach.error.protocol":
    "O Have I Been Pwned enviou uma resposta em que o Mosaic não confia, então algumas senhas não foram verificadas. Tente de novo mais tarde.",
  "breach.error.cancelled":
    "A verificação foi interrompida. Os resultados já concluídos foram mantidos.",
  "breach.consent.title": "Verificar suas senhas em vazamentos de dados conhecidos?",
  "breach.consent.body":
    "O Mosaic transforma cada senha em um código embaralhado que não pode ser revertido (um hash) e envia ao Have I Been Pwned (api.pwnedpasswords.com) apenas os 5 primeiros caracteres desse código. O serviço fica sabendo o seu endereço IP e esses trechos de 5 caracteres. Ele nunca vê uma senha, o seu e-mail nem para qual site é uma senha. Os resultados ficam neste dispositivo.",
  "breach.consent.detail":
    "Muitas outras senhas têm o mesmo trecho de 5 caracteres, então a resposta não revela a sua. Ainda assim, é uma solicitação a outro serviço, por isso fica desativada até você concordar. Você pode desativá-la a qualquer momento.",
  "breach.consent.agree": "Ativar e verificar agora",
  "breach.consent.decline": "Agora não",
} satisfies Record<keyof typeof en, string>;
