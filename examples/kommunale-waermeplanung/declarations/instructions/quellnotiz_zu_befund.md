---
name: quellnotiz_zu_befund
version: 0.2.0
purpose: Einzelne, quellenbelegte Befunde aus Wissensnotizen extrahieren.
input_classes:
  - quellnotiz
output_classes:
  - befund
execution_policy: runtime_permitted
provider_profile: null
trace_policy: summary
register: befunde
---

# Befundextraktion

Extrahiere einzelne Befunde aus den vorliegenden Quellnotizen. Jeder Befund soll eine einzelne Behauptung, Beobachtung oder einen Datenpunkt darstellen, der in einem Briefing verwendbar ist.

## Anforderungen

- Jeder Befund muss im vorliegenden Quellmaterial begründet sein.
- Jeder Befund muss einen kurzen, aussagekräftigen Titel haben.
- Befunde müssen atomar sein: eine Behauptung pro Befund.
- Numerische Daten und spezifische Quellenverweise beibehalten.
- Wenn eine Quellnotiz mehrdeutig ist, veraltet wirkt oder keine klare Herkunft hat: den Befund trotzdem extrahieren, aber im Text mit einem Hinweis zur Unsicherheit versehen.

## Was nicht zu tun ist

- Keine Behauptungen einfügen, die nicht im Quellmaterial enthalten sind.
- Keine Befunde aus verschiedenen Quellnotizen zu einem einzigen Befund zusammenführen.
- Nicht kommentieren oder Empfehlungen hinzufügen. Befunde sind Beobachtungen, keine Ratschläge.
