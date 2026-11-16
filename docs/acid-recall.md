# ACID RECALL

![Logo ACID RECALL](../assets/brand/acid-recall.png)

**Le souvenir revient. Les machines repartent.**

Deux basses acid. Deux boîtes à rythmes. Un sampler. Des contrôleurs sous les doigts.
ACID RECALL fait évoluer le rack d’OpenRB-338 vers une pratique plus directe : jouer,
générer des idées, découper des voix et faire dialoguer les machines.

## L’histoire — retrouver le premier déclic

ReBirth a été le premier logiciel de musique que j’ai installé sur mon PC.
Bien avant ce projet, il y a donc eu ce premier contact : deux lignes de basse,
des boîtes à rythmes et la possibilité de commencer à faire de la musique sur ordinateur.

Alors, lorsque j’ai découvert qu’un projet recréait ReBirth, j’ai été trop content.
OpenRB-338 existait. Les deux 303, la 808, la 909 : cette petite famille de machines
pouvait de nouveau tourner sur un ordinateur actuel.

J’ai commencé à jouer avec. Puis j’ai regardé mon vieux contrôleur Evolution UC-33e,
qui fonctionne toujours très bien. La première envie était toute simple : retrouver
ces machines avec mes faders et mes potentiomètres.

C’est comme ça que tout a commencé. Un fork, du MIDI, quelques affectations.
Je ne suis pas développeur de logiciel : j’avance avec une assistance IA pour traduire
mes envies de musicien en fonctions, les essayer, relever ce qui gêne et recommencer.

Très vite, une idée en a amené une autre. Générer des lignes de basse qui restent
en harmonie. Faire répondre une 303 à l’autre. Ajouter un arpégiateur sur chacune.
Garder huit pas sur l’une et seize sur l’autre. Donner de l’espace au kick.

Puis sont arrivés le sampler, les voix découpées au rythme des hats, la reverb à
convolution et une interface graphite pensée pour avoir les commandes sous les yeux.
Le logiciel a commencé à suivre ma façon de faire de la musique aujourd’hui.

ACID RECALL est né de ce mouvement : retrouver un premier déclic et lui donner une suite.
Le souvenir reste là. Maintenant, j’ai envie de voir ce qu’on peut en faire.

## Ce que ce fork ajoute à OpenRB-338

Le point de départ est le travail de réimplémentation d’OpenRB-338. Les évolutions
ci-dessous décrivent notre fork ; elles ne prétendent pas recréer ce socle de zéro.

| Domaine | Évolutions du fork |
|---|---|
| Contrôle physique | MIDI Learn sous Windows, plusieurs entrées simultanées, reprise douce et calibration par contrôleur. |
| Routages | Import/export des affectations pour retrouver son installation sur un autre ordinateur. |
| Génération | Choix Techno, IDM et Break ; notes, accents, slides et Undo. |
| Dialogue des 303 | LINK harmonique avec des longueurs indépendantes, par exemple 8 et 16 pas. |
| Arpèges | Un arpégiateur par TB, à partir d’un accord ou des notes du pattern ; application directe avec ARP. |
| Mixage | Sidechain piloté par une voix choisie de la 808 ou de la 909, avec profondeur indépendante sur chaque TB. |
| Hats 909 | CH et OH indépendantes pour les notes et les niveaux. |
| Espace | Reverb stéréo à convolution, espaces synthétiques et import d’impulsions WAV/AIFF. |
| Sampler | Huit pistes, séquences jusqu’à 64 mesures, WAV/AIFF/MP3, points IN/OUT et Reverse. |
| Jeu du sampler | One Shot, Auto Cut, Gate et Loop ; Solo/Mute accessibles sur chaque ligne. |
| Mouvement | Gater piloté par une voix TR, maintien synchronisé au tempo, lissage et sidechain optionnel. |
| Tranche sampler | ON/OFF assignable en MIDI, volume global, vue-mètre stéréo, panoramique, distortion, delay et reverb globale ; sends individuels conservés. |
| Interface | Rack graphite, réglages TR visibles, sampler intégré et colonne FX permanente. |
| Sessions | Projets `.orb338` transportant les samples, l’impulsion et les réglages Studio. |

Les contrôles du sampler, de la reverb et du sidechain disposent d’affectations MIDI.
L’UC-33e a servi de point de départ ; les affectations ne lui sont pas réservées.

## Une identité pour cette nouvelle étape

**ACID** pour les basses, les séquenceurs et le terrain musical du projet.
**RECALL** pour le souvenir qui revient et l’envie de rappeler les machines au jeu.

Le monogramme AR intègre une flèche de rappel. Le graphite et le vert acide reprennent
l’esprit du rack : une interface sombre, des états actifs visibles et peu de décoration.

### Présentation courte

ACID RECALL est un fork d’OpenRB-338 qui prolonge l’esprit de ReBirth avec du contrôle
MIDI, des générateurs de patterns, deux arpégiateurs et un sampler intégré. Un rack
graphite pour retrouver les machines d’hier et construire des sessions d’aujourd’hui.

### Texte de lancement

ReBirth a été le premier logiciel de musique que j’ai installé sur mon PC.
Des années plus tard, découvrir OpenRB-338 m’a donné envie de rejouer avec ces machines.
J’ai commencé par connecter mon vieux UC-33e. Puis sont venus les patterns en harmonie,
les arpégiateurs, le sidechain, le sampler et une nouvelle interface.
Ce fork s’appelle désormais ACID RECALL. Le souvenir revient. Les machines repartent.

## Où en est le projet ?

Le projet reste en évolution. Les builds Windows de cette branche permettent d’essayer
les fonctions décrites ici. La synchronisation MIDI Clock IN/OUT avec d’autres machines
est une piste pour la suite : **elle n’est pas implémentée à ce stade**. L’entrée MIDI
actuelle sert au contrôle des paramètres ; elle ne signifie pas que le logiciel est
déjà maître ou esclave d’une horloge MIDI externe.

Le sampler charge les fichiers en mémoire. Les limites de taille et les détails
d’utilisation sont décrits dans le [guide Studio](studio.md).
ACID RECALL reste un rack et un séquenceur, avec un projet qui grandit au fil des sessions.

## Origines et crédits

ACID RECALL est développé dans le [fork de smldms](https://github.com/smldms/openrb-338),
à partir d’[OpenRB-338 de pingw33n](https://github.com/pingw33n/openrb-338).
Le travail sur le moteur de réimplémentation et la compatibilité historique appartient
à ce projet amont et à ses contributeurs. Le code conserve sa licence GPL-3.0-only.

ReBirth RB-338, le logiciel de Propellerhead, est à l’origine de cette aventure.
