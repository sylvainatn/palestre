# D-004 — Contrat entre la VM et son environnement

| Champ | Valeur |
|-------|--------|
| Identifiant | D-004 |
| Date | 2026-11-13 |
| Auteurs | Brouillon rédigé avec Codex à la demande de Shany ; validation de l'équipe en attente |
| Statut | `proposée` |
| Commits | Aucun commit pour cette entrée à ce stade |
| Clauses | B.2, B.3, B.5, E.2 à E.4, K.3 à K.6 ; README §3 |

## 1. Contexte

La VM n'est pas encore implémentée. Une seule machine doit pouvoir utiliser les réponses du corpus, une arène locale ou des rappels réseau synchrones. L'environnement fournit les valeurs de `SENSE` et `RAND` et l'issue de `ACT`. Le trait ne doit dépendre ni de TCP, ni de JSON, ni de la grille.

Il faut distinguer trois situations : une action échouée (`false`, le tour continue), une faute normative (`Fault`, le tour se termine avec un résultat) et un rappel impossible à terminer (transport ou protocole, exécution interrompue sans résultat, K.6). On ne peut pas transformer une coupure en dix-septième variante de `Fault`.

## 2. Options envisagées

Les signatures suivantes sont des propositions documentaires, pas du code livré.

### Option A — Trois méthodes synchrones avec erreur technique associée

```rust
pub trait Env {
    type Error;

    fn sense(&mut self, sensor: u8)
        -> Result<Result<i64, Fault>, Self::Error>;
    fn act(&mut self, kind: u8, arg: i64)
        -> Result<bool, Self::Error>;
    fn rand(&mut self) -> Result<i64, Self::Error>;
}
```

Pour `sense`, `Ok(Ok(value))` fournit une valeur ; `Ok(Err(fault))` fournit une faute de tour ; `Err(error)` interrompt l'exécution. Le double `Result` est volontaire : le réseau peut recevoir une réponse `SENSED` valide portant une faute de tour (K.3), ou échouer avant d'obtenir une réponse valide.

* Avantages : une méthode correspond à une instruction ; `act` ne peut pas renvoyer une valeur entière arbitraire et `rand` ne peut pas renvoyer une réponse d'action. Aucun `Box`, aucune allocation imposée et aucune dépendance réseau dans la VM.
* Limites : le double `Result` doit être expliqué ; `Machine::run` devra propager `E::Error`, ce qui implique une généricité limitée à l'environnement.

La VM aurait une sortie de forme `Result<TurnResult, E::Error>` avec `E: Env`. `TurnResult` contiendrait notamment `fault: Option<Fault>` : une faute de tour reste un résultat ; une erreur technique est le `Err` extérieur. La structure complète du résultat sera définie avec la VM.

### Option B — Une méthode avec requêtes et réponses typées

```rust
pub enum EnvRequest {
    Sense { sensor: u8 },
    Act { kind: u8, arg: i64 },
    Rand,
}

pub enum EnvResponse {
    Sense(Result<i64, Fault>),
    Act(bool),
    Rand(i64),
}

pub trait Env {
    type Error;

    fn request(&mut self, request: EnvRequest)
        -> Result<EnvResponse, Self::Error>;
}
```

* Avantages : point de passage unique, pratique pour enregistrer les rappels et rapprocher requêtes et trames réseau ; pas d'allocation imposée non plus.
* Limites : une réponse `Act` à une requête `Sense` est représentable. Il faut un contrôle supplémentaire et un traitement typé de cette incohérence, même dans l'arène locale. Cette forme rapproche davantage l'interface VM du protocole réseau.
* Erreurs : les fautes normatives restent dans `Sense`, les erreurs techniques dans le `Result` extérieur. Une réponse du mauvais genre est une erreur d'adaptateur/protocole, pas une faute supplémentaire de la VM.

Une variante plus courte, où les trois méthodes renverraient seulement `Result<_, Fault>`, est écartée de la recommandation : elle ne représente pas correctement les erreurs de transport exigées par K.6.

## 3. Ce que l'assistant IA a proposé

Codex recommande l'option A pour sa correspondance directe avec les instructions et l'impossibilité de mélanger les genres de réponses. Le type associé sert uniquement à conserver les erreurs propres à chaque environnement ; il ne justifie pas d'autres abstractions génériques.

Contrat proposé avec cette option :

* Les méthodes sont synchrones et prennent `&mut self` pour avancer dans un script, échanger sur une connexion ou modifier une arène.
* La VM applique le cycle B.2 et les contrôles locaux avant tout rappel : une instruction refusée ne doit produire aucun appel extérieur. Elle rejette notamment une action hors de `0..=3` et le second `ACT` du tour, en respectant l'ordre normatif des vérifications. L'environnement ne gère pas le drapeau « déjà agi » de la VM.
* `sensor` conserve le numéro `u8` du bytecode. L'environnement répond à la demande ou fournit une faute de tour. L'adaptateur réseau valide `SENSED` selon K.3, dont le rejet de `BadHeader` ; il ne réduit pas arbitrairement les réponses autorisées à la seule variante `BadSensor`.
* `act` renvoie un booléen, converti en `0` ou `1` par la VM. Une direction invalide de `MOVE` est un échec ordinaire ; une action valide, réussie ou échouée, consomme l'unique tentative du tour. La VM consigne `{kind, arg, ok}` après la réponse ; `BadAction` ne consigne aucune action.
* `rand` fournit un `i64` sans tirage caché dans la VM ou dans le serveur réseau. L'environnement local fournit son générateur ; les tests fournissent des valeurs scriptées.
* Aucun rappel ne reçoit les piles ni la mémoire de la VM. L'arène assure l'instantané et la restauration du tour selon E.2 ; la VM conserve ses écritures antérieures à une faute dans son résultat.
* Les rappels respectent l'ordre des instructions, sans anticipation ni répétition automatique. Un incident réseau interrompt l'exécution sans `RESULT` ; `net` décide de l'éventuel `ERROR` et de la fermeture selon K.8.
* Le trait n'impose aucune allocation pendant un tour. Chaque implémentation devra aussi respecter cette contrainte et réserver les tampons nécessaires avant l'exécution.

Utilisation et validation prévues :

| Environnement | État et réponses | Erreur technique |
|---------------|------------------|------------------|
| Tests | Liste de réponses scriptées, position courante, contrôle du genre et des paramètres des appels | Réponse absente, genre ou paramètres inattendus ; contrôle des réponses restantes après le tour |
| Réseau | Connexion et tampons prévus, aller-retour K.6, validation des trames K.3/K.4 | E/S, coupure, trame invalide ou message inattendu, sans conversion en `Fault` |
| Arène | Monde et agent courant, capteurs E.3, actions E.4, source d'aléa locale | `std::convert::Infallible` si aucun échec technique n'est possible |

Lors de l'implémentation, vérifier les rappels et leur ordre, la propagation d'une faute `SENSED`, l'échec ordinaire d'une action, l'absence de rappel après rejet local, et l'interruption sans résultat en cas de rappel interrompu. Ces scénarios sont à implémenter avec la VM et les adaptateurs ; aucun test d'exécution n'est revendiqué ici.

## 4. Ce que nous avons décidé, et pourquoi

**Proposition à valider : option A et contrat décrit en section 3.** Elle paraît la plus simple pour les trois consommateurs et distingue les fautes de tour des incidents techniques.

Aucune décision humaine ni signature n'est consignée par ce brouillon. L'équipe doit renseigner les noms, la date réelle, l'option retenue et ses raisons, puis mettre le statut à `adoptée` avant l'implémentation. L'index du journal devra refléter cette adoption.

## 5. Ce qui nous ferait changer d'avis

* Le double `Result` entraîne des confusions réelles : envisager un enum explicite distinguant valeur et faute, en conservant l'erreur technique séparée.
* Un besoin concret de suspendre et reprendre l'exécution impose une nouvelle interface ; ne pas ajouter d'async sans respecter les contraintes du sujet.
* Une modification de la spec change les rappels ou leurs erreurs : revoir les signatures et les scénarios avant le code correspondant.

## Révisions

_(aucune ; entrée ouverte avant l'implémentation de la VM)_
