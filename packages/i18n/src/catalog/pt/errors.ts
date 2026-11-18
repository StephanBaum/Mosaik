/**
 * Brazilian Portuguese UI strings: errors. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { errors as en } from "../en/errors.ts";

export const errors = {
  "error.notImplemented": "Isto ainda não está disponível. Chegará em uma versão futura do Mosaic.",
  "error.noAccount":
    "Ainda não há uma conta neste dispositivo. Crie uma ou entre com o seu Kit de Emergência.",
  "error.accountExists": "Este dispositivo já tem uma conta do Mosaic. Desbloqueie-a.",
  "error.locked": "O Mosaic está bloqueado. Desbloqueie para continuar.",
  "error.alreadyUnlocked": "O Mosaic já está desbloqueado. Você pode continuar de onde parou.",
  "error.wrongCredentials":
    "Essa senha mestra ou Chave Secreta não funcionou. Verifique o Caps Lock e o layout do teclado e tente de novo.",
  "error.secretKeyRequired":
    "Este dispositivo não tem a sua Chave Secreta. Digite-a a partir do seu Kit de Emergência.",
  "error.secretKeyMalformed":
    "Isso não é uma Chave Secreta. A sua começa com “M1-” e está no seu Kit de Emergência.",
  "error.secretKeyMistyped":
    "Essa Chave Secreta tem um erro de digitação. Compare-a com o seu Kit de Emergência, caractere por caractere.",
  "error.emptyPassword": "A senha mestra está vazia. Digite-a para continuar.",
  "error.quickUnlockUnavailable":
    "O desbloqueio rápido não está configurado neste dispositivo. Desbloqueie com a sua senha mestra.",
  "error.storeInUse":
    "O Mosaic já está aberto neste computador. Feche a outra janela e tente de novo.",
  "error.storeUnavailable":
    "O Mosaic não conseguiu abrir o arquivo do seu cofre neste dispositivo. Verifique se o disco está disponível e tem espaço livre e reinicie o Mosaic.",
  "error.vaultNotFound":
    "Esse cofre não existe mais. Ele pode ter sido excluído em outro dispositivo.",
  "error.vaultUnavailable":
    "Este cofre não pode ser aberto nesta sessão. Desbloqueie-o com a chave de hardware dele ou sincronize e tente de novo.",
  "error.itemNotFound":
    "Esse item não existe mais. Ele pode ter sido movido ou excluído em outro dispositivo.",
  "error.itemDeleted":
    "Esse item foi excluído permanentemente e não pode ser aberto. Escolha outro item.",
  "error.sameVault": "O item já está nesse cofre. Escolha um cofre diferente.",
  "error.noOtp": "Este item não tem código de uso único. Edite o item para adicionar um.",
  "error.fieldEmpty":
    "Esse campo está vazio, então não há nada para copiar ou mostrar. Edite o item para preenchê-lo.",
  "error.invalidInput":
    "Algo neste formulário não é válido. Verifique os campos destacados e tente de novo.",
  "error.sshKeyInvalid":
    "Esta não é uma chave privada OpenSSH. Cole a chave inteira, incluindo as linhas BEGIN e END.",
  "error.reauthRequired": "Esta alteração exige a sua senha mestra. Digite-a para confirmar.",
  "error.attachmentTooLarge": "Esse arquivo tem mais de 50 MiB. Escolha um arquivo menor.",
  "error.attachmentQuota":
    "Seus arquivos usariam mais do que o 1 GiB disponível. Remova alguns anexos e tente de novo.",
  "error.attachmentNotFound": "Este arquivo não faz mais parte do item. Recarregue o item.",
  "error.attachmentUnavailable":
    "Este arquivo não pode ser aberto neste dispositivo. Ele pode ainda não ter sido sincronizado ou estar danificado.",
  "error.attachmentFileUnreadable": "Não é possível ler esse arquivo. Escolha outro arquivo.",
  "error.attachmentSaveFailed": "Não foi possível salvar o arquivo ali. Escolha outra pasta.",
  "error.attachmentPreviewUnsupported":
    "Não há pré-visualização para este arquivo. Use Salvar como para abri-lo.",
  "error.attachmentPreviewTooLarge":
    "Esta imagem é grande demais para pré-visualizar. Use Salvar como para abri-la.",
  "error.clipboardUnavailable":
    "O Mosaic não conseguiu usar a área de transferência. Feche apps que possam estar usando-a e tente de novo.",
  "error.importFileUnreadable":
    "O Mosaic não conseguiu ler esse arquivo. Verifique se ele ainda existe e não está aberto em outro app e tente de novo.",
  "error.importDeleteNotAllowed":
    "Importe o arquivo primeiro. O Mosaic só exclui uma exportação depois que os itens dela estão no seu cofre.",
  "error.importDeleteFailed":
    "Não foi possível excluir o arquivo. Ele pode estar aberto em outro app ou ser somente leitura. Exclua-o você mesmo e esvazie a lixeira.",
  "error.importFormatInvalid":
    "Esse arquivo não está no formato que você escolheu. Escolha o formato correspondente ou exporte o arquivo de novo.",
  "error.importPreviewExpired":
    "Esta importação foi cancelada. Escolha o arquivo de novo para recomeçar.",
  "error.sshRequestExpired":
    "Essa solicitação SSH já terminou. Execute o comando de novo se ainda precisar dela.",
  "error.sshAgentUnavailable":
    "O agente SSH não pôde ser iniciado. Verifique se nenhum outro agente SSH está em execução e ative-o de novo.",
  "error.unlockDelayed":
    "Muitas tentativas erradas seguidas. Aguarde alguns segundos e tente de novo.",
  "error.tooManyAttempts":
    "Muitas respostas erradas. O Mosaic foi bloqueado. Desbloqueie-o com a sua senha mestra para continuar.",
  "error.sshAllowTooSoon":
    "Isso foi rápido demais para ler a solicitação. Confira a chave e o programa e escolha Permitir de novo.",
  "error.browserRequestExpired":
    "Essa solicitação do navegador já terminou. Peça de novo pela extensão se ainda precisar dela.",
  "error.browserAllowTooSoon":
    "Isso foi rápido demais para ler a solicitação. Confira os detalhes e escolha de novo.",
  "error.browserPairingNotFound":
    "Esse navegador não está mais conectado. Reabra as Configurações para ver a lista atual.",
  "error.browserHostMissing":
    "Esta cópia do Mosaic não tem um conector de navegador instalado, então não há nada para reparar. Instale o Mosaic com o instalador dele para conectar um navegador.",
  "error.agentOriginInvalid":
    "Os agentes só podem enviar uma chave a um site que comece com https:// e indique apenas o site, como https://api.example.com. Confira o endereço.",
  "error.agentBindingInvalid":
    "O Mosaic não pode usar esta configuração para agentes. Confira o campo, os sites e como a chave é enviada.",
  "error.agentValueUnusable":
    "Este valor é curto demais ou tem uma quebra de linha, então os agentes não podem usá-lo. Uma chave precisa de pelo menos 12 caracteres em uma única linha.",
  "error.agentVaultShared":
    "Os agentes só podem usar itens de um cofre que pertença a você e não seja compartilhado. Mova o item para um dos seus cofres primeiro.",
  "error.agentAccessOff":
    "O acesso de agentes está desativado. Ative-o em Configurações, em Agentes.",
  "error.agentRequestExpired":
    "Essa solicitação do agente já terminou. O agente pode pedir de novo se ainda precisar dela.",
  "error.agentAllowTooSoon":
    "Isso foi rápido demais para ler a solicitação. Confira os detalhes e escolha de novo.",
  "error.agentPairingNotFound":
    "Esse agente não está mais conectado. Reabra as Configurações para ver a lista atual.",
  "error.agentEndpointUnavailable":
    "O Mosaic não conseguiu abrir a conexão que os agentes usam. Feche outras cópias do Mosaic e tente de novo.",
  "error.kitNotVerified":
    "Salve primeiro o novo Kit de Emergência e confirme o último grupo dele. O Mosaic não altera nada antes disso.",
  "error.kitGroupMismatch":
    "Esse não é o último grupo da nova Chave Secreta. Confira o novo Kit de Emergência.",
  "error.rotationInProgress":
    "Outra troca de chaves ainda está aberta. Conclua-a com Continuar troca de chaves ou descarte-a primeiro.",
  "error.rotationNone": "Não há nenhuma troca de chaves para continuar.",
  "error.rotationNotFound":
    "Essa troca de senha não está mais aberta. Ela pode ter sido fechada em outro dispositivo ou no navegador. Reabra o item para ver em que pé ela está.",
  "error.rotationLimit":
    "Já há quatro trocas de senha abertas para este item. Conclua ou feche uma delas no item primeiro.",
  "error.rotationOpen":
    "Este login tem uma troca de senha aberta. Restaure-o da lixeira, depois escolha Funcionou ou Não funcionou no item. Só então você poderá excluí-lo permanentemente.",
  "error.rotationNotSupported":
    "O Mosaic não consegue orientar esta troca. Ela exige um login com endereço https. Troque a senha no site você mesmo.",
  "error.historyNotFound":
    "Essa senha anterior não está mais no histórico. Reabra o item para ver a lista atual.",
  "error.originNotRelated":
    "Essa página não pertence ao site deste login, então o Mosaic não a adicionou. Edite os endereços do item se ela deveria pertencer.",
  "error.breachConsentRequired":
    "A verificação de vazamentos está desativada. Ative-a e concorde primeiro com o que ela envia.",
  "error.breachRunning":
    "Uma verificação de vazamentos já está em andamento. Aguarde o fim dela ou interrompa-a primeiro.",
  "error.rotationTooFar":
    "Esta troca já chegou à sua conta, então não pode ser descartada. Continue-a para concluir.",
  "error.rotationJumpTooLarge":
    "Este dispositivo está atrasado demais em relação aos outros para se atualizar. Configure-o de novo: remova a conta deste dispositivo e entre outra vez com o seu Kit de Emergência.",
  "error.rotationNotApplied":
    "A troca nunca chegou à sua conta, então nada foi alterado. Comece de novo ou descarte-a.",
  "error.secretKeyUnchanged":
    "Essa Chave Secreta não funciona mais. Use a do Kit de Emergência que você salvou no dispositivo que fez a alteração.",
  "error.passwordUnchanged": "A nova senha mestra precisa ser diferente da atual.",
  "error.profileDowngrade":
    "Esse nível não é mais forte do que o que a sua conta usa agora. Escolha um mais alto.",
  "error.profileTooSlow":
    "Desbloquear com esse nível levaria mais de dois segundos neste dispositivo. Escolha um nível mais baixo ou atualize mesmo assim.",
  "error.rekeyNeedsCleanSync":
    "Renovar as chaves exige que a sincronização esteja em dia e sem avisos. Sincronize agora, veja qualquer aviso e tente de novo.",
  "error.rekeyMemberKeyMismatch":
    "Você ainda não comparou o código de segurança de todos em um cofre compartilhado, então esse cofre não pode ser renovado. Compare os códigos nas configurações de compartilhamento do cofre e tente de novo.",
  "error.rekeyBusy":
    "Seus cofres continuaram mudando enquanto o Mosaic trabalhava. Aguarde um momento e tente de novo.",
  "error.rekeyOutcomeUnknown":
    "O Mosaic não conseguiu confirmar se a sua conta aceitou as novas chaves. Escolha Continuar troca de chaves para descobrir.",
  "error.rekeyNeedsServer":
    "Renovar as chaves exige o seu servidor ou pasta de sincronização, e não foi possível acessá-lo. Verifique a conexão e tente de novo.",
  "error.rekeyNoAccountFile":
    "Nada na sua pasta de sincronização abre com esta senha e Chave Secreta. Confira o novo Kit de Emergência.",
  "error.syncAccountRekeyed":
    "As chaves da sua conta foram renovadas em outro dispositivo. Use a Chave Secreta do novo Kit de Emergência para continuar.",
  "error.syncEpochRollback":
    "O seu backend de sincronização mostra chaves de conta mais antigas do que as deste dispositivo. Nada foi alterado.",
  "error.syncRekeyRefused":
    "As chaves que o seu backend de sincronização oferece não são as seguintes às deste dispositivo. Nada foi alterado. Verifique os seus outros dispositivos.",
  "error.pickerStale": "Esta lista está desatualizada. Clique no campo de novo.",
  "error.pickerTooSoon": "Aguarde um momento e confirme de novo.",
  "error.programFillUnavailable":
    "O preenchimento em programas não está disponível neste dispositivo.",
  "error.busy": "O Mosaic ainda está concluindo a última etapa. Aguarde um momento e tente de novo.",
  "error.internal":
    "Algo inesperado interrompeu esta ação. Tente de novo e reinicie o Mosaic se continuar acontecendo.",
  "error.importTooLarge":
    "Este arquivo é grande ou complexo demais para importar. Exporte menos itens por vez e tente de novo.",
  "error.importUnsupported":
    "O Mosaic não consegue ler esta versão do formato. Exporte de novo em um formato atual (no KeePass: KDBX 4 com AES ou ChaCha20) e tente de novo.",
  "error.importEncrypted":
    "Esta exportação é protegida por senha. Exporte de novo sem criptografia, importe e depois exclua o arquivo.",
  "error.sshAgentEndpointInUse":
    "Outro agente SSH já está em execução (no Windows, o serviço OpenSSH Authentication Agent). Pare-o e ative de novo o agente SSH do Mosaic.",
  "error.cancelled": "Cancelado. Nada foi alterado.",
  "error.quickUnlockExpired":
    "O desbloqueio rápido expirou. Desbloqueie com a sua senha mestra e ative-o de novo em Configurações.",
  "error.quickUnlockInvalidated":
    "O desbloqueio rápido parou de funcionar neste dispositivo e foi desativado. Desbloqueie com a sua senha mestra.",
  "error.passwordRequired":
    "Digite a sua senha mestra desta vez. Depois disso, o desbloqueio rápido volta a funcionar.",
  "error.hardwareKeyUnsupported":
    "Esta chave de segurança ou este sistema não pode ser usado com o Mosaic.",
  "error.hardwareKeyNotFound": "Nenhuma chave de segurança encontrada. Insira-a e tente de novo.",
  "error.hardwareKeyTimeout": "A chave não foi tocada a tempo. Tente de novo.",
  "error.hardwareKeyPinRequired": "Digite o PIN da sua chave de segurança.",
  "error.hardwareKeyPinInvalid":
    "PIN incorreto. Após várias tentativas erradas, a chave se bloqueia sozinha.",
  "error.hardwareKeyPinBlocked":
    "Esta chave está bloqueada após PINs errados demais. Redefina-a com a ferramenta do fabricante; os registros dela serão perdidos.",
  "error.hardwareKeyNoPin": "Defina antes um PIN nesta chave, com a ferramenta do fabricante.",
  "error.hardwareKeyNotRecognized":
    "Esta chave não está registrada aqui. Tente outra chave ou a sua senha mestra.",
  "error.hardwareKeyLimit":
    "Você registrou o máximo de chaves permitido (8). Remova uma primeiro.",
  "error.hardwareKeyLast":
    "Esta é a única chave do cofre. Registre outra antes de removê-la.",
  "error.itemTypeNotAllowed": "Este cofre guarda apenas códigos de uso único.",
  "error.otpScanUnsupported":
    "A leitura da tela não está disponível neste sistema. Cole uma captura de tela.",
  "error.otpQrNotFound":
    "Nenhum código QR encontrado. Certifique-se de que o código inteiro está visível e tente de novo.",
  "error.otpQrNotOtp":
    "Este código QR não é um código de configuração de verificação em duas etapas. Procure o código na página de configuração de 2FA do site.",
  "error.otpQrMigration":
    "Este é um código de exportação do Google Authenticator, que o Mosaic ainda não consegue ler. Configure cada conta de novo no site dela ou digite a chave de configuração.",
  "error.otpQrMultiple":
    "Mais de um código de configuração está visível. Mostre apenas o que você quer e tente de novo.",
  "error.otpQrInvalid":
    "Este código de configuração usa ajustes que o Mosaic não oferece suporte. Digite a chave de configuração.",
  "error.otpUriInvalid": "Este não é um link otpauth:// válido. Copie o link inteiro de novo.",
  "error.otpKeyInvalid":
    "Esta chave não é válida. Ela usa apenas as letras de A a Z e os dígitos de 2 a 7; verifique se há erros de digitação.",
  "error.otpImageUnreadable":
    "O Mosaic não consegue ler esta imagem. Use um arquivo PNG ou JPEG.",
  "error.otpImageTooLarge":
    "Esta imagem é grande demais. Use uma captura de tela só do código QR.",
  "error.otpClipboardNoImage":
    "A área de transferência não tem imagem. Faça uma captura de tela do código QR e cole-a aqui.",
  "error.otpPendingExpired": "O código lido não está mais disponível. Leia-o de novo.",
  "error.syncNotConfigured":
    "A sincronização ainda não está configurada neste dispositivo. Configure-a antes em Configurações.",
  "error.syncAlreadyConfigured":
    "A sincronização já está configurada. Desconecte primeiro para usar outro servidor ou pasta.",
  "error.syncServerUnreachable":
    "O Mosaic não conseguiu acessar o servidor. Verifique o endereço e a sua conexão com a internet e tente de novo.",
  "error.syncServerInvalid":
    "Isto não parece um servidor Mosaic. Verifique o endereço; ele deve ser parecido com https://sync.example.com.",
  "error.syncServerInsecure":
    "O Mosaic só se conecta a servidores por https, para que ninguém no caminho possa ler ou alterar os seus dados. Use um endereço https://.",
  "error.syncServerMismatch":
    "Este não é o servidor com o qual esta conta sincroniza. Verifique o endereço.",
  "error.syncServerIncompatible":
    "Este servidor executa uma versão que não é compatível com o seu Mosaic. Atualize o Mosaic ou peça ao dono do servidor para atualizá-lo.",
  "error.syncRegistrationClosed":
    "Este servidor não está aceitando novas contas de sincronização. Entre em uma conta existente ou fale com o dono do servidor.",
  "error.syncInviteInvalid":
    "Este código de convite não é válido ou já foi usado. Confira-o ou peça um novo ao dono do servidor.",
  "error.syncAccountExists":
    "Você já tem uma conta de sincronização neste servidor. Escolha Entrar.",
  "error.syncAccountMismatch":
    "A conta de sincronização nesse servidor pertence a outra conta do Mosaic. Verifique o endereço e o seu e-mail.",
  "error.syncTwoFactorRequired":
    "Esta conta de sincronização exige um código do seu app autenticador. Digite o código de 6 dígitos.",
  "error.syncTwoFactorInvalid":
    "Esse código não está correto ou expirou. Digite o código de 6 dígitos atual do seu app autenticador.",
  "error.syncSignedOut":
    "O servidor desconectou este dispositivo. Entre de novo para continuar sincronizando. Suas alterações estão seguras neste dispositivo.",
  "error.syncRateLimited": "Tentativas demais. Aguarde alguns minutos e tente de novo.",
  "error.syncQuotaExceeded":
    "O servidor ou a pasta não tem mais espaço para os seus dados. Libere espaço ou peça mais ao dono do servidor.",
  "error.syncFolderUnavailable":
    "O Mosaic não pode usar esta pasta. Verifique se ela existe, se o seu serviço de nuvem terminou de sincronizá-la e se o Mosaic pode gravar nela.",
  "error.syncFolderNotEmpty":
    "Esta pasta contém outros arquivos, então o Mosaic não gravará nela. Escolha uma pasta vazia ou crie uma nova.",
  "error.syncFolderNoAccount":
    "Esta pasta não contém dados de sincronização desta conta. Escolha a pasta com a qual o seu outro dispositivo sincroniza.",
  "error.syncFolderDamaged":
    "Alguns arquivos de sincronização desta pasta estão danificados. Deixe o seu serviço de nuvem terminar de sincronizar ou restaure a pasta de um backup e tente de novo.",
  "error.syncPasswordOutcomeUnknown":
    "Não conseguimos confirmar se o servidor alterou a sua senha mestra. Guarde as duas senhas até a sincronização confirmar qual vale.",
  "error.syncWebauthnUnavailable":
    "O seu servidor não pode usar uma chave de segurança para isto agora, ou esta conta não tem nenhuma. Use um código de app autenticador.",
  "error.syncWebauthnLimit":
    "Esta conta já tem o máximo de chaves de segurança que o servidor permite. Remova uma que você não usa mais e tente de novo.",
  "error.syncHandoffExpired": "A etapa no navegador expirou. Comece-a de novo.",
  "error.syncHandoffFailed":
    "Códigos errados ou verificações com falha demais no navegador. Comece a etapa de novo.",
  "error.syncPasswordNeedsServer":
    "Alterar a sua senha mestra exige o seu servidor de sincronização, e não foi possível acessá-lo. Verifique a sua conexão com a internet e tente de novo.",
  "error.syncDeviceIsCurrent":
    "Você não pode remover o dispositivo que está usando. Use Desconectar.",
  "error.syncChoiceExpired": "A escolha da pasta expirou. Escolha a pasta de novo.",
  "error.syncFreshLoginExpired":
    "A verificação da sua senha expirou. Digite a sua senha mestra de novo.",
  "error.syncPasswordRollback": "O servidor ofereceu uma senha mestra mais antiga. Verifique o servidor.",
  "error.syncPasswordChanged":
    "Sua senha mestra foi alterada no servidor ou na pasta. Digite a nova abaixo.",
  "error.sharingNotHosted":
    "O compartilhamento exige um servidor Mosaic. Conecte um antes nas configurações de Sincronização.",
  "error.setupCodeNeedsServer": "Um celular só pode entrar em uma conta que sincroniza.",
  "error.sharingContactUnverified":
    "Compare primeiro o código de segurança desta pessoa. O Mosaic só compartilha com pessoas que você verificou.",
  "error.sharingKeyChanged":
    "A chave desta pessoa mudou ou o código não confere. Procure-a de novo e compare o novo código antes de prosseguir.",
  "error.sharingRoleForbidden":
    "Sua função neste cofre não permite isso. Fale com o proprietário ou um gerente.",
  "error.sharingInviteExists":
    "Já há um convite aguardando para esta pessoa e este cofre. Retire-o primeiro se quiser enviar um novo.",
  "error.sharingInviteStale":
    "O convite mudou nesse meio tempo, por exemplo após uma troca de chave. Abra-o de novo.",
  "error.sharingInviteExpired":
    "Este convite expirou ou foi retirado. Peça a quem enviou que convide você de novo.",
  "error.sharingInviteUnreadable":
    "Este convite não pode ser aberto. Ele pode estar danificado ou não ser para esta conta. Peça a quem enviou que convide você de novo.",
  "error.sharingInviteNotInRoster":
    "A lista de membros do cofre não corresponde a este convite, então o Mosaic não o abriu. Peça a quem enviou que convide você de novo.",
  "error.sharingOwnerCannotLeave":
    "O proprietário não pode sair de um cofre. Remova as outras pessoas.",
  "error.sharingAlreadyMember": "Esta pessoa já é membro do cofre.",
  "error.sharingReinviteBlocked":
    "Você removeu esta pessoa recentemente. Você poderá convidá-la de novo depois que a chave do cofre mudar. Sincronize e tente de novo.",
  "error.sharingInviteLimit":
    "Há convites demais aguardando. Espere as respostas ou retire alguns.",
  "error.sharingVaultKnown":
    "Este dispositivo já tem esse cofre, então o Mosaic não aceitou uma chave para ele. Pergunte a quem enviou qual cofre foi compartilhado.",
  "error.syncHasSharedVaults":
    "Você não pode trocar de servidor enquanto houver cofres compartilhados neste dispositivo. Saia dos cofres compartilhados, ou peça aos proprietários que removam você, primeiro.",
  "error.emergencyWaiting":
    "Há uma solicitação aguardando, então isto não pode mudar agora. Recuse-a ou aguarde.",
  "error.emergencyCooldown":
    "Sua última solicitação foi recusada. Você poderá pedir de novo após o período de espera de um dia.",
  "error.emergencyNotGranted":
    "Não há nada a solicitar: nenhuma solicitação está aberta ou a pessoa não concedeu acesso a você.",
  "error.shortcutUnavailable":
    "Outro programa já usa este atalho ou o Mosaic não consegue registrá-lo. Escolha outra combinação.",
  "error.shortcutUnsupported":
    "Seu sistema não permite que apps definam um atalho global. Abra o Mosaic pela janela dele.",
} satisfies Record<keyof typeof en, string>;
