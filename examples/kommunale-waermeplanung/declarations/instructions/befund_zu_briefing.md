---
name: befund_zu_briefing
version: 0.2.0
purpose: Validierte Befunde zu einer strukturierten Briefingkarte zusammenfassen.
input_classes:
  - befund
output_classes:
  - briefingkarte
execution_policy: runtime_permitted
provider_profile: null
trace_policy: summary
register: briefings
---

# Briefingkarte erstellen

Fasse die vorliegenden Befunde zu einer strukturierten Briefingkarte für die interne Verwendung zusammen.

## Ausgabestruktur

Die Briefingkarte muss die folgenden Abschnitte enthalten:

### Zusammenfassung
2-3 Sätze zu den zentralen Themen über alle Befunde hinweg.

### Gesicherte Befunde
Liste jedes Befundes, der durch Quellmaterial gut belegt ist. Für jeden Befund angeben, aus welchem Befund er stammt (nach Titel oder Referenz). Thematisch verwandte Befunde gruppieren.

### Handlungsimplikationen
Was bedeuten diese Befunde für Planung oder Entscheidungsfindung? Konkret und praxisnah formulieren.

### Offene Fragen
Was bleibt unklar, umstritten oder unzureichend belegt? Befunde einbeziehen, die im Extraktionsschritt als unsicher oder veraltet markiert wurden.

### Gesperrtes oder unbrauchbares Material
Befunde auflisten, die in der vorliegenden Form nicht verwendet werden können, und erklären warum. Dieser Abschnitt kann leer sein, wenn alle Befunde verwendbar sind.

## Regeln

- Die Briefingkarte darf nur auf den vorliegenden Befunden basieren.
- Nicht direkt auf Quellnotizen verweisen. Die Briefing-Stufe erhält ausschließlich Befunde, nicht das Rohmaterial. Das ist beabsichtigt.
- Die Briefingkarte unter 500 Wörtern halten.
- Klare Sprache verwenden, verständlich für nicht-technische Entscheider.
