/**
 * Brazilian Portuguese UI strings: app. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { app as en } from "../en/app.ts";

export const app = {
  "app.name": "Mosaic",
  "app.preAudit": "Software pré-auditoria: ainda não revisado por um auditor de segurança externo.",
  "app.previewTag": "Prévia",
  "app.previewBuild":
    "Versão de prévia, não é uma versão final. Sem assinatura, pré-auditoria, apenas para testes.",
  "action.cancel": "Cancelar",
  "action.continue": "Continuar",
  "action.retry": "Tentar de novo",
  "action.copy": "Copiar",
  "action.reveal": "Mostrar",
  "action.hide": "Ocultar",
  "action.lock": "Bloquear",
  "action.details": "Detalhes",
  "sync.offline":
    "Não foi possível acessar o seu servidor de sincronização. Suas alterações estão salvas neste dispositivo e serão sincronizadas quando você voltar a ficar online.",
  "app.loading": "Abrindo o Mosaic…",
  "app.skipToContent": "Ir para o conteúdo",
  "action.save": "Salvar",
  "action.close": "Fechar",
  "action.showPassword": "Mostrar senha",
  "action.hidePassword": "Ocultar senha",
  "action.back": "Voltar",
  "action.confirm": "Confirmar",
  "action.edit": "Editar",
  "action.delete": "Mover para a Lixeira",
  "action.restore": "Restaurar",
  "action.purge": "Excluir permanentemente",
  "action.generate": "Gerar",
  "action.useThis": "Usar esta",
  "action.done": "Concluir",
  "nav.main": "Navegação principal",
  "nav.vault": "Cofre",
  "nav.security": "Segurança",
  "nav.generator": "Gerador",
  "nav.settings": "Configurações",
  "nav.trash": "Lixeira",
  "nav.breadcrumb": "Trilha de navegação",
  "nav.securityCount": "{count, plural, one {# alerta para revisar} other {# alertas para revisar}}",
  "sync.status.local": "Armazenado neste dispositivo",
  "sync.status.upToDate": "Sincronizado",
  "sync.status.attention": "Requer atenção",
  "sync.status.syncing": "Sincronizando…",
  "sync.status.offline": "Offline",
  "sync.status.error": "Falha na sincronização",
  "action.moreActions": "Mais ações",
  "action.regenerate": "Gerar de novo",
} satisfies Record<keyof typeof en, string>;
