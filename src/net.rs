//! Protocole d'exécution distante — annexe K.
//!
//! Responsabilités de ce module :
//!
//! * le cadrage K.2 : `type` (1 octet), `len` (`u32` gros-boutiste), `payload` ; les bornes
//!   de `len` sont vérifiées **avant toute allocation** ;
//! * les treize messages de K.3, en JSON sauf `SUBMIT` qui porte les octets bruts du `.pbc` ;
//! * la machine d'états par connexion de K.4 : `Greeting` → `Idle` ⇄ `Executing` ;
//! * les rappels `SENSE` / `ACT` / `RAND` vers le client (K.6), qui tient lieu de monde ;
//! * les codes d'erreur de K.8 et leur effet sur la connexion.
//!
//! Le serveur est séquentiel par défaut (README §2) ; la concurrence est un bonus, en `std` seul.
