/**
 * Brazilian Portuguese UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "Segurança",
  "insights.intro":
    "As verificações são feitas neste dispositivo. Última verificação em {checkedAt, date, medium} às {checkedAt, time, short}, em {count, plural, one {# item} other {# itens}}.",
  "insights.checkAgain": "Verificar de novo",
  "insights.summary": "Resumo",
  "insights.fixFirst": "Corrija estes primeiro",
  "insights.showAll": "Mostrar {count, plural, one {# alerta} other {todos os # alertas}}",
  "insights.empty":
    "Nenhum problema encontrado. O Mosaic verifica de novo sempre que você abre esta página.",
  "insights.kind.weak": "Fraca",
  "insights.kind.weak.description": "Fácil de adivinhar",
  "insights.kind.reused": "Reutilizada",
  "insights.kind.reused.description": "{groups, plural, one {# grupo} other {# grupos}}",
  "insights.kind.old": "Antiga (mais de 1 ano)",
  "insights.kind.old.description": "Não alterada há mais de um ano",
  "insights.kind.missingTotp": "Sem 2FA",
  "insights.kind.missingTotp.description": "O site oferece, mas você não configurou",
  "insights.kind.insecureUrl": "HTTP sem proteção",
  "insights.kind.insecureUrl.description": "Envia a sua senha sem criptografia",
  "insights.kind.breached": "Em um vazamento",
  "insights.kind.breached.description": "Encontrada pela verificação de vazamentos opcional",
  "insights.finding.breached":
    "Esta senha aparece em um vazamento de dados conhecido. Troque-a agora, em todos os lugares onde você a usou.",
  "insights.finding.weak": "Fácil de adivinhar. Troque-a por uma senha gerada.",
  "insights.finding.reused":
    "Também usada em {count, plural, one {# outro item} other {# outros itens}}. Dê a cada site a sua própria senha.",
  "insights.finding.old":
    "Não alterada há mais de um ano. Troque-a se você ainda usa esta conta.",
  "insights.finding.missingTotp":
    "{site} oferece códigos de verificação em duas etapas. Ative-os lá e salve o código no Mosaic.",
  "insights.finding.insecureUrl":
    "Usa HTTP simples: {url}. Mude para https:// se o site oferecer suporte.",
  "insights.severity.high": "Alta",
  "insights.severity.medium": "Média",
  "insights.severity.low": "Baixa",
  "insights.action.changePassword": "Trocar senha",
  "insights.action.setUpTotp": "Configurar 2FA",
  "insights.action.reviewUrl": "Revisar",
  "insights.attribution.twoFactor": "Dados de 2FA: 2FA Directory (2fa.directory), licença MIT.",
  "insights.counts":
    "{count, plural, =0 {Nenhum problema} one {# problema} other {# problemas}}",
  "insights.documentation": "Como ativar: {url}",
  "insights.actionFor": "{action}: {title}",
} satisfies Record<keyof typeof en, string>;
