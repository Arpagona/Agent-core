# Spécifications fonctionnelles — Projet Artisans du Sud

**Version :** 1.0
**Auteur :** Cabinet Conseil Numérique Régional
**Statut :** Brouillon — En attente de validation client

---

## 1. Fonctionnalités e-commerce

### 1.1 Catalogue produits
- Affichage par catégorie (poterie, vannerie, textile)
- Filtres : matière, prix, couleur, artisan
- Recherche plein texte avec suggestions
- Fiches produits avec photos (5-8 par produit), description, prix, stock

### 1.2 Panier et commande
- Panier persistant (compte obligatoire)
- Devis automatique pour commandes > 500 €
- Options de livraison : Colissimo, Point Relais, retrait atelier
- Paiement : Carte bancaire, Virement, PayPal

### 1.3 Gestion des stocks multi-atelier
- Interface de mise à jour par atelier
- Alertes seuil bas (10% du stock)
- Historique des mouvements de stock

## 2. Fonctionnalités de personnalisation

### 2.1 Configuration produit
- Sélecteur de couleur (palette limitée selon produit)
- Sélecteur de dimensions (options prédéfinies)
- Champ texte libre pour motif/détail
- Aperçu en temps réel (simulation image)

## 3. Programme de fidélité

### 3.1 Règles de cumul
- 1 point par 10 € d'achat
- Points doublés sur les produits en promotion
- 50 points = 5 € de réduction
- Cumul sans limite de durée

## 4. Contraintes techniques

- Pas de base de données externe (hébergement mutualisé)
- Compatibilité SEO (maintien des URLs existantes)
- Responsive design obligatoire
- Accessibilité RGAA niveau A
- Conformité RGPD pour les données clients
