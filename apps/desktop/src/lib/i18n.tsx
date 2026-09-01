import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { getSettings, setSettings } from "./void";

export type Locale = "en" | "fr";
export type Params = Record<string, string | number>;

const en = {
  "common.cancel": "Cancel",
  "common.close": "Close",
  "common.copy": "Copy",
  "common.copied": "Copied ✓",
  "common.you": "you",
  "titlebar.connectedTor": "Connected through Tor",
  "titlebar.connectingTor": "Connecting to Tor",
  "titlebar.minimize": "Minimize",
  "titlebar.maximize": "Maximize",
  "titlebar.close": "Close",
  "rail.dms": "Direct messages",
  "rail.createGroup": "Create a group",
  "rail.settings": "Settings",
  "rail.membersOnline": "some members are online",
  "rail.membersOffline": "no member online",
  "channel.home": "Home",
  "channel.welcome": "welcome",
  "channel.myIdentity": "my identity",
  "channel.messages": "Messages",
  "channel.addPeer": "Add a peer via an invite link",
  "channel.removePeer": "Remove this peer",
  "channel.noPeers": "no peer added",
  "channel.viaTor": "via Tor",
  "channel.textChannels": "Text channels",
  "channel.general": "general",
  "channel.members": "{n} members",
  "members.inOrbit": "In orbit — {n}",
  "members.inTheVoid": "In the void — {n}",
  "members.owner": "owner",
  "members.member": "member",
  "members.youPeerOnion": "you · onion peer",
  "members.fingerprintVerified": "Verified fingerprint: {fp}",
  "members.outgoing": "outgoing",
  "members.incoming": "incoming",
  "tor.network": "Tor network",
  "tor.starting": "Starting the embedded Tor relay…",
  "tor.circuits": "Establishing circuits",
  "tor.connected": "Connected through Tor",
  "tor.socks": "socks {addr}",
  "tor.failed": "Tor network failure",
  "messages.today": "Today",
  "messages.yesterday": "Yesterday",
  "messages.delivered": "✓✓ delivered",
  "messages.sent": "✓ sent",
  "messages.awaitingRelay": "⟳ awaiting relay",
  "chat.emptyTitle": "Start of your conversation with {name}",
  "chat.emptyHint":
    "End-to-end encrypted messages (X25519 + ChaCha20-Poly1305), routed through the Tor network. The first words float in the void…",
  "chat.writeTo": "Write to {name}",
  "chat.offlineRelay":
    "{name} is offline — the message will be relayed until they return",
  "chat.torConnecting": "Connecting to Tor…",
  "chat.footnote":
    "Enter to send · Shift+Enter for a new line · E2E encrypted · offline delivery through relay peers",
  "chat.copyOnion": "Copy the onion address",
  "chat.ping": "ping",
  "composer.send": "Send",
  "composer.record": "Record a voice note",
  "composer.pause": "Pause",
  "composer.resume": "Resume",
  "composer.stop": "Stop and preview",
  "composer.micDenied": "Microphone unavailable — check Windows permissions for Void.",
  "friend.requests": "Requests",
  "friend.accept": "Accept",
  "friend.decline": "Decline",
  "notify.friendRequest": "{name} added you on Void",
  "profile.title": "{name}",
  "profile.loading": "Loading profile…",
  "profile.bio": "Bio",
  "profile.address": "Void address",
  "profile.copyAddress": "Copy address",
  "profile.message": "Message",
  "identity.bio": "Bio",
  "identity.bioPlaceholder": "a few words floating in the void…",
  "identity.status": "Status",
  "identity.statusPlaceholder": "orbiting…",
  "identity.accent": "Accent color",
  "identity.changeAvatar": "Change avatar",
  "identity.removeAvatar": "Remove",
  "identity.avatarTooLarge": "Image too large after compression — try a simpler image (max 64 KB)",
  "identity.avatarInvalid": "Unreadable image file",
  "ctx.copy": "Copy",
  "ctx.paste": "Paste",
  "ctx.selectAll": "Select all",
  "ctx.copyAddress": "Copy address",
  "ctx.copyLink": "Copy link",
  "ctx.cut": "Cut",
  "update.downloading": "Downloading… {percent}%",
  "update.installing": "Installing…",
  "update.downloadSize": "{size} MB",
  "settings.microphone": "Microphone",
  "settings.microphoneDefault": "System default",
  "settings.speaker": "Speaker",
  "settings.speakerDefault": "System default",
  "settings.playback": "Playback follows system default",
  "group.header": "{total} members · {online} online",
  "group.invite": "+ invite",
  "group.leave": "leave",
  "group.leaveConfirm":
    "Leave “{name}”? The local history will be deleted.",
  "group.welcomeTitle": "Welcome to {name}",
  "group.emptyHint":
    "Encrypted group conversation (shared key, P2P fan-out over Tor). Offline members receive through relay peers for 7 days.",
  "group.writeIn": "Write in {name}",
  "group.footnote":
    "Peer-to-peer fan-out · offline members receive through relays",
  "welcome.channelName": "welcome",
  "welcome.p2pTor": "P2P over Tor",
  "welcome.title": "Welcome to the void, {name}",
  "welcome.subtitle":
    "Peer-to-peer messaging over Tor. No server, no IP address exposed — just you, your peers, and the space between you.",
  "welcome.signal": "Your signal in the void — send it to a friend",
  "welcome.copyLink": "Copy link",
  "welcome.showQr": "Show QR",
  "welcome.hideQr": "Hide QR",
  "welcome.peers": "Peers",
  "welcome.addPeer": "+ Add a peer",
  "welcome.noPeers":
    "No peer yet. Exchange your signals (link or QR code), then add each other here — conversations are end-to-end encrypted.",
  "welcome.clickPeer":
    "Click a peer in the left column to open the conversation — end-to-end encrypted.",
  "welcome.createGroup": "+ Create a group",
  "welcome.onboarding": "Your display name is not set yet.",
  "welcome.configureIt": "Set it up",
  "identity.title": "Your Void identity",
  "identity.subtitle":
    "On Void, your identity is an onion address: no account, no server. Share it so other peers can reach you.",
  "identity.displayName": "Display name",
  "identity.displayNamePlaceholder": "your nickname",
  "identity.save": "Save",
  "identity.onionAddress": "Void address (.onion)",
  "identity.fingerprint": "fingerprint: {fp}",
  "identity.onionGenerating": "generating identity…",
  "identity.recovery": "Recovery phrase",
  "identity.saved": "saved ✓",
  "identity.recoveryDesc":
    "24 words that regenerate your identity on any machine. Nobody can recover it for you.",
  "identity.paperWarning":
    "Write it on paper, never in a file or a photo.",
  "identity.showPhrase": "Show the phrase",
  "identity.confirmed": "I saved my phrase",
  "identity.restore": "Restore an identity",
  "identity.restoreDesc":
    "Paste a recovery phrase (24 words) to take back an existing identity. The current identity will be replaced.",
  "identity.restorePlaceholder": "word1 word2 word3 … word24",
  "identity.restoreButton": "Restore this identity",
  "identity.restoring": "Restoring + restarting Tor…",
  "identity.restoreConfirm":
    "Restore this identity?\n\nYour current identity and its .onion address will be permanently replaced.",
  "addPeer.title": "Add a peer",
  "addPeer.subtitle":
    "Paste an invite signal void://invite received from your contact.",
  "addPeer.peerPrefix": "peer {id}…",
  "addPeer.validOnion": "valid onion ✓",
  "addPeer.adding": "Adding…",
  "addPeer.addButton": "Add peer",
  "createGroup.title": "Create a group",
  "createGroup.subtitle":
    "A constellation: an encrypted conversation key shared with each invited member, over a pairwise encrypted channel.",
  "createGroup.name": "Group name",
  "createGroup.namePlaceholder": "the constellation",
  "createGroup.members": "Members — only online peers can be invited",
  "createGroup.noOnline":
    "No peer online. You can invite later from the group.",
  "createGroup.creating": "Creating…",
  "createGroup.createButton": "Create group",
  "inviteGroup.title": "Invite into “{name}”",
  "inviteGroup.none": "No online peer to invite (besides current members).",
  "inviteGroup.inviting": "inviting…",
  "inviteGroup.inviteAction": "invite →",
  "settings.title": "Settings",
  "settings.notifications": "Notifications",
  "settings.notificationsHint":
    "Notify of new messages when Void is not in the foreground",
  "settings.autostart": "Launch Void at startup",
  "settings.autostartHint": "Open Void automatically at Windows logon",
  "settings.language": "Language",
  "settings.update": "Update",
  "settings.updateAvailable": "Void {version} is available",
  "settings.upToDate": "Up to date (Void {version})",
  "settings.checking": "Searching…",
  "settings.installing": "Downloading + installing…",
  "settings.updateError":
    "Could not reach github.com — check your connection. (This check goes outside Tor, deliberately manual.)",
  "settings.installError": "Download or installation failed.",
  "settings.checkGithub": "Check published versions on GitHub",
  "settings.installRestart": "Install and restart",
  "settings.check": "Check",
  "settings.manualNote":
    "Strictly manual check: it contacts github.com directly (outside Tor) only when you click.",
  "settings.about": "About",
  "settings.version": "Version",
  "settings.transport": "Transport",
  "settings.transportValue": "Embedded Tor (onion v3)",
  "settings.relayQueue": "Relay queue",
  "settings.relayQueueValue": "{n} envelope(s)",
  "settings.data": "Data",
  "error.title": "Something went wrong",
  "error.details": "technical details (paste into a GitHub issue)",
  "error.reload": "Reload Void",
  "notify.newMessage": "New message",
  "notify.group": "Void — group",
} as const;

export type I18nKey = keyof typeof en;

const fr: Record<I18nKey, string> = {
  "common.cancel": "Annuler",
  "common.close": "Fermer",
  "common.copy": "Copier",
  "common.copied": "Copié ✓",
  "common.you": "vous",
  "titlebar.connectedTor": "Connecté via Tor",
  "titlebar.connectingTor": "Connexion Tor en cours",
  "titlebar.minimize": "Réduire",
  "titlebar.maximize": "Agrandir",
  "titlebar.close": "Fermer",
  "rail.dms": "Messages privés",
  "rail.createGroup": "Créer un groupe",
  "rail.settings": "Réglages",
  "rail.membersOnline": "des membres sont en ligne",
  "rail.membersOffline": "aucun membre en ligne",
  "channel.home": "Accueil",
  "channel.welcome": "bienvenue",
  "channel.myIdentity": "mon identité",
  "channel.messages": "Messages",
  "channel.addPeer": "Ajouter un pair via un lien d'invitation",
  "channel.removePeer": "Retirer ce pair",
  "channel.noPeers": "aucun pair ajouté",
  "channel.viaTor": "via Tor",
  "channel.textChannels": "Canaux texte",
  "channel.general": "général",
  "channel.members": "{n} membres",
  "members.inOrbit": "En orbite — {n}",
  "members.inTheVoid": "Dans le vide — {n}",
  "members.owner": "propriétaire",
  "members.member": "membre",
  "members.youPeerOnion": "vous · pair oignon",
  "members.fingerprintVerified": "Empreinte vérifiée : {fp}",
  "members.outgoing": "sortant",
  "members.incoming": "entrant",
  "tor.network": "Réseau Tor",
  "tor.starting": "Démarrage du relais Tor embarqué…",
  "tor.circuits": "Établissement des circuits",
  "tor.connected": "Connecté via Tor",
  "tor.socks": "socks {addr}",
  "tor.failed": "Échec du réseau Tor",
  "messages.today": "Aujourd'hui",
  "messages.yesterday": "Hier",
  "messages.delivered": "✓✓ délivré",
  "messages.sent": "✓ envoyé",
  "messages.awaitingRelay": "⟳ en attente de relais",
  "chat.emptyTitle": "Début de votre conversation avec {name}",
  "chat.emptyHint":
    "Messages chiffrés de bout en bout (X25519 + ChaCha20-Poly1305), transit par le réseau Tor. Les premiers mots flottent dans le vide…",
  "chat.writeTo": "Écrire à {name}",
  "chat.offlineRelay":
    "{name} est hors ligne — le message sera relayé jusqu'à son retour",
  "chat.torConnecting": "Connexion Tor en cours…",
  "chat.footnote":
    "Enter pour envoyer · Maj+Enter pour un saut de ligne · chiffré E2E · livraison hors-ligne par pairs relais",
  "chat.copyOnion": "Copier l'adresse oignon",
  "chat.ping": "ping",
  "composer.send": "Envoyer",
  "composer.record": "Enregistrer une note vocale",
  "composer.pause": "Mettre en pause",
  "composer.resume": "Reprendre",
  "composer.stop": "Arrêter et écouter",
  "composer.micDenied": "Micro indisponible — vérifiez les permissions Windows de Void.",
  "friend.requests": "Demandes",
  "friend.accept": "Accepter",
  "friend.decline": "Refuser",
  "notify.friendRequest": "{name} vous a ajouté sur Void",
  "profile.title": "{name}",
  "profile.loading": "Chargement du profil…",
  "profile.bio": "Bio",
  "profile.address": "Adresse Void",
  "profile.copyAddress": "Copier l'adresse",
  "profile.message": "Écrire",
  "identity.bio": "Bio",
  "identity.bioPlaceholder": "quelques mots flottant dans le vide…",
  "identity.status": "Statut",
  "identity.statusPlaceholder": "en orbite…",
  "identity.accent": "Couleur d'accent",
  "identity.changeAvatar": "Changer l'avatar",
  "identity.removeAvatar": "Retirer",
  "identity.avatarTooLarge": "Image trop volumineuse après compression — essayez une image plus simple (64 Ko max)",
  "identity.avatarInvalid": "Fichier image illisible",
  "ctx.copy": "Copier",
  "ctx.paste": "Coller",
  "ctx.selectAll": "Tout sélectionner",
  "ctx.copyAddress": "Copier l'adresse",
  "ctx.copyLink": "Copier le lien",
  "ctx.cut": "Couper",
  "update.downloading": "Téléchargement… {percent}%",
  "update.installing": "Installation…",
  "update.downloadSize": "{size} Mo",
  "settings.microphone": "Microphone",
  "settings.microphoneDefault": "Système (défaut)",
  "settings.speaker": "Haut-parleur",
  "settings.speakerDefault": "Système (défaut)",
  "settings.playback": "La lecture suit le système",
  "group.header": "{total} membres · {online} en ligne",
  "group.invite": "+ inviter",
  "group.leave": "quitter",
  "group.leaveConfirm": "Quitter « {name} » ? L'historique local sera supprimé.",
  "group.welcomeTitle": "Bienvenue dans {name}",
  "group.emptyHint":
    "Conversation de groupe chiffrée (clé partagée, fan-out P2P via Tor). Les membres hors ligne reçoivent via pairs relais pendant 7 jours.",
  "group.writeIn": "Écrire dans {name}",
  "group.footnote": "Fan-out pair-à-pair · les membres hors ligne reçoivent via relais",
  "welcome.channelName": "bienvenue",
  "welcome.p2pTor": "P2P via Tor",
  "welcome.title": "Bienvenue dans le vide, {name}",
  "welcome.subtitle":
    "Messagerie pair-à-pair au-dessus de Tor. Aucun serveur, aucune adresse IP exposée — juste vous, vos pairs, et l'espace entre eux.",
  "welcome.signal": "Votre signal dans le vide — envoyez-le à un ami",
  "welcome.copyLink": "Copier le lien",
  "welcome.showQr": "Afficher le QR",
  "welcome.hideQr": "Masquer le QR",
  "welcome.peers": "Pairs",
  "welcome.addPeer": "+ Ajouter un pair",
  "welcome.noPeers":
    "Aucun pair pour l'instant. Échangez vos signaux (lien ou QR code), puis ajoutez-les ici — les conversations sont chiffrées de bout en bout.",
  "welcome.clickPeer":
    "Cliquez sur un pair dans la colonne de gauche pour ouvrir la conversation — chiffrée de bout en bout.",
  "welcome.createGroup": "+ Créer un groupe",
  "welcome.onboarding": "Votre nom d'affichage n'est pas encore défini.",
  "welcome.configureIt": "Configurez-le",
  "identity.title": "Votre identité Void",
  "identity.subtitle":
    "Sur Void, votre identité est une adresse oignon : pas de compte, pas de serveur. Partagez-la pour que d'autres pairs vous contactent.",
  "identity.displayName": "Nom d'affichage",
  "identity.displayNamePlaceholder": "votre pseudo",
  "identity.save": "Enregistrer",
  "identity.onionAddress": "Adresse Void (.onion)",
  "identity.fingerprint": "empreinte : {fp}",
  "identity.onionGenerating": "génération de l'identité…",
  "identity.recovery": "Phrase de récupération",
  "identity.saved": "sauvegardée ✓",
  "identity.recoveryDesc":
    "24 mots qui régénèrent votre identité sur n'importe quelle machine. Personne ne peut la récupérer pour vous.",
  "identity.paperWarning": "Écrivez-la sur papier, jamais dans un fichier ou une photo.",
  "identity.showPhrase": "Afficher la phrase",
  "identity.confirmed": "J'ai sauvegardé ma phrase",
  "identity.restore": "Restaurer une identité",
  "identity.restoreDesc":
    "Collez une phrase de récupération (24 mots) pour reprendre une identité existante. L'identité actuelle sera remplacée.",
  "identity.restorePlaceholder": "mot1 mot2 mot3 … mot24",
  "identity.restoreButton": "Restaurer cette identité",
  "identity.restoring": "Restauration + redémarrage Tor…",
  "identity.restoreConfirm":
    "Restaurer cette identité ?\n\nVotre identité actuelle et son adresse .onion seront remplacées définitivement.",
  "addPeer.title": "Ajouter un pair",
  "addPeer.subtitle":
    "Collez un signal d'invitation void://invite reçu de votre contact.",
  "addPeer.peerPrefix": "pair {id}…",
  "addPeer.validOnion": "oignon valide ✓",
  "addPeer.adding": "Ajout…",
  "addPeer.addButton": "Ajouter le pair",
  "createGroup.title": "Créer un groupe",
  "createGroup.subtitle":
    "Une constellation : une clé de conversation chiffrée partagée avec chaque membre invité, via un canal pairwise chiffré.",
  "createGroup.name": "Nom du groupe",
  "createGroup.namePlaceholder": "la constellation",
  "createGroup.members": "Membres — seuls les pairs en ligne sont invitables",
  "createGroup.noOnline": "Aucun pair en ligne. Vous pourrez inviter plus tard depuis le groupe.",
  "createGroup.creating": "Création…",
  "createGroup.createButton": "Créer le groupe",
  "inviteGroup.title": "Inviter dans « {name} »",
  "inviteGroup.none": "Aucun pair en ligne à inviter (hors membres actuels).",
  "inviteGroup.inviting": "invitation…",
  "inviteGroup.inviteAction": "inviter →",
  "settings.title": "Réglages",
  "settings.notifications": "Notifications",
  "settings.notificationsHint":
    "Notifier des nouveaux messages quand Void n'est pas au premier plan",
  "settings.autostart": "Lancer Void au démarrage",
  "settings.autostartHint": "Ouvre Void automatiquement à l'ouverture de session Windows",
  "settings.language": "Langue",
  "settings.update": "Mise à jour",
  "settings.updateAvailable": "Void {version} est disponible",
  "settings.upToDate": "À jour (Void {version})",
  "settings.checking": "Recherche…",
  "settings.installing": "Téléchargement + installation…",
  "settings.updateError":
    "Impossible de joindre github.com — vérifiez votre connexion. (Ce contrôle passe hors Tor, délibérément manuel.)",
  "settings.installError": "Le téléchargement ou l'installation a échoué.",
  "settings.checkGithub": "Vérifier les versions publiées sur GitHub",
  "settings.installRestart": "Installer et redémarrer",
  "settings.check": "Vérifier",
  "settings.manualNote":
    "Vérification strictement manuelle : elle contacte github.com directement (hors Tor) uniquement quand vous cliquez.",
  "settings.about": "À propos",
  "settings.version": "Version",
  "settings.transport": "Transport",
  "settings.transportValue": "Tor embarqué (oignon v3)",
  "settings.relayQueue": "File relais",
  "settings.relayQueueValue": "{n} enveloppe(s)",
  "settings.data": "Données",
  "error.title": "Une erreur est survenue",
  "error.details": "détails techniques (à copier dans un issue GitHub)",
  "error.reload": "Recharger Void",
  "notify.newMessage": "Nouveau message",
  "notify.group": "Void — groupe",
};

const dictionaries: Record<Locale, Record<I18nKey, string>> = { en, fr };

function interpolate(template: string, params?: Params): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_, key: string) =>
    params[key] !== undefined ? String(params[key]) : `{${key}}`
  );
}

interface I18nContextValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: I18nKey, params?: Params) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>("en");

  useEffect(() => {
    getSettings()
      .then((settings) => setLocaleState(settings.language === "fr" ? "fr" : "en"))
      .catch(() => undefined);
  }, []);

  const setLocale = useCallback((next: Locale) => {
    setLocaleState(next);
    getSettings()
      .then((settings) => setSettings({ ...settings, language: next }))
      .catch(() => undefined);
  }, []);

  const t = useCallback(
    (key: I18nKey, params?: Params) =>
      interpolate(dictionaries[locale][key] ?? key, params),
    [locale]
  );

  return (
    <I18nContext.Provider value={{ locale, setLocale, t }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error("useI18n must be used within I18nProvider");
  }
  return context;
}

export function dateLocale(locale: Locale): string {
  return locale === "fr" ? "fr-FR" : "en-US";
}
