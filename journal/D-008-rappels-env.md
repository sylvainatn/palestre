# D-008 — Branchement des rappels Env

| Champ | Valeur |
|-------|--------|
| Identifiant | D-008 |
| Date | 2026-11-15 |
| Auteurs | Proposition Codex à la demande de Shany ; revue humaine en attente |
| Statut | `proposée` |
| Commits | Aucun |
| Clauses | B.2–B.3, E.3–E.4, K.6 ; ticket 06 |

## 1. Contexte

Les instructions internes sont intégrées. Le ticket 06 demande de déléguer les interactions à Env. D-004 décrit déjà deux options mais reste proposée ; cette implémentation réalise son option A dans le périmètre demandé sans consigner une adoption humaine.

## 2. Options envisagées

- Conserver la fonction injectée publique et fournir un adaptateur Env : préserve les anciens appelants mais leur permet de remplacer les contrôles locaux des interactions.
- Faire recevoir à Machine::run un environnement mutable : garantit les contrôles de la VM, au prix de la migration de la signature publique. Le cycle avec fonction injectée reste privé pour les tests isolés du gaz.

## 3. Ce que l'assistant IA a proposé

Codex implémente la seconde option et les trois méthodes synchrones de D-004 : sense rend une valeur ou une faute de tour dans un Result intérieur, act rend un booléen, rand rend un i64. L'erreur extérieure est technique et interrompt sans TurnResult. Aucun tirage n'a lieu dans la VM. Les adaptateurs doivent préparer leurs tampons avant le tour et respecter l'absence d'allocation pendant les rappels.

Le gaz est prélevé par le cycle avant les contrôles et les rappels. SENSE et RAND vérifient la place sur la pile avant de rappeler l'hôte. Le numéro du capteur est transmis tel quel : l'environnement décide si le capteur existe. ACT contrôle d'abord l'action déjà enregistrée, puis le numéro 0..=3, puis dépile l'argument. Après la réponse, il enregistre la tentative, réussie ou échouée, et empile exactement 0 ou 1. Une erreur technique ne consigne aucune réponse d'action.

TRACE conserve son tampon réservé et son ordre d'émission. Les fautes gardent la mémoire, l'action et les traces déjà produites ; les transitoires sont vidés au prochain tour. Les tests scriptés vérifient exactement le genre et les paramètres des rappels ainsi que l'épuisement du script, sans imposer de transcript alloué à la VM.

## 4. Ce que nous avons décidé et pourquoi

Proposition implémentée sur demande du ticket ; revue et adoption humaine en attente. Machine::run reçoit désormais &mut impl Env. Les tests isolés du cycle utilisent run_cycle privé ; les tests des interactions passent par l'interface publique. Aucun adaptateur réseau ni monde local n'est livré par ce ticket.

La spécification ne départage pas explicitement toutes les fautes simultanées d'ACT : la priorité locale retenue est AlreadyActed, BadAction, StackUnderflow. Les cas isolés sont couverts. D-004 reste proposée ; cette entrée documente l'implémentation concrète sans modifier son historique.

## 5. Ce qui nous ferait changer d'avis

Une clarification normative impose une autre priorité de fautes ou un rappel malgré une pile pleine : ajuster les contrôles et les tests. L'intégration réseau révèle une ambiguïté du double Result : envisager un enum explicite, en conservant la séparation entre fautes normatives et incidents techniques.
