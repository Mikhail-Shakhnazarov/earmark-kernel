# Kommunale Wärmeplanung — Wissensbriefing

Diese Demo zeigt, wie aus heterogenen Wissensnotizen eine belastbare Briefingkarte entsteht,
ohne dass Kontext als unstrukturierte Chat-Historie weitergereicht wird.

## Das Problem

In Organisationen der Energie- und Wärmeplanung entstehen laufend Notizen, Workshopprotokolle,
Rückmeldungen und Berichte. Das Material ist vorhanden, aber die darin enthaltenen Aussagen
werden oft nicht als wiederverwendbare Wissenseinheiten gesichert. Reine KI-Zusammenfassungen
ohne Zwischenstufe erschweren Nachvollziehbarkeit und Qualitätsprüfung.

## Was diese Demo zeigt

Die Pipeline arbeitet in zwei Stufen: `quellnotiz -> befund -> briefingkarte`. Zuerst werden
atomare Befunde mit Herkunft und Unsicherheitsmarkierung extrahiert. Danach wird ausschließlich
auf Basis dieser Befunde eine strukturierte Briefingkarte erstellt. Jede Stufe sieht nur die
für sie deklarierte Arbeitsoberfläche (bounded context).

## Das Quellkorpus

Das Quellkorpus umfasst fünf Notizen mit bewusst unterschiedlicher Qualität: ein
Workshopprotokoll, ein datengestützter Fortschrittsbericht, eine interne Pilotnotiz, eine
Stakeholder-Rückmeldung und eine unklare Altnotiz. Die Qualitätsvariation ist absichtlich,
um robuste Befundextraktion und sauberen Umgang mit Unsicherheit zu demonstrieren.

## Demo ausführen

Vom Repository-Root aus:

```bash
cd examples/kommunale-waermeplanung

em system register declarations/systems/system.yaml
em declare validate declarations/systems/system.yaml
em system activate sys_waermeplanung_briefing

em deposit seed/notiz_1_workshop.md --class quellnotiz --title "Workshopprotokoll Wärmeplanung"
em deposit seed/notiz_2_fortschrittsbericht.md --class quellnotiz --title "Fortschrittsbericht Auszug"
em deposit seed/notiz_3_pilotprojekt.md --class quellnotiz --title "Pilotprojekt Datenintegration"
em deposit seed/notiz_4_stakeholder.md --class quellnotiz --title "Stakeholder-Rückmeldung"
em deposit seed/notiz_5_unklare_altnotiz.md --class quellnotiz --title "Unklare Altnotiz Fernwärme"

em workflow run waermeplanung_briefing --provider local_mock

em list --class befund
em list --class briefingkarte
em show --class briefingkarte --latest
```

## Ergebnisse prüfen

Prüfen Sie die erzeugte `briefingkarte` und vergleichen Sie sie mit
`expected-output/briefingkarte.md`. Achten Sie besonders darauf, dass die Synthese nur Befunde
verwendet und unsicheres Material sichtbar ausweist, statt es stillschweigend einzubauen.

## Was diese Demo demonstriert

- Begrenzter Kontext: Jede Stufe sieht nur deklarierte Eingaben.
- Persistente Befunde: Extrahierte Aussagen bleiben als Objekte im Store erhalten.
- Gesteuerter Handoff: Die Briefing-Stufe arbeitet ausschließlich auf Befunden.
- Sichtbare Unsicherheit: Schwaches Material wird markiert statt versteckt.
- Nachvollziehbare Herkunft: Befunde bleiben auf Quellnotizen rückführbar.

## Über Earmark

Earmark ist ein deklarationsorientiertes Laufzeitsystem für gesteuerte KI-Ausführung und
dauerhafte Wissensobjekte. Die Demo zeigt, wie aus verteilten Notizen eine prüfbare,
weiterverwendbare Wissensgrundlage für Entscheidungen entsteht.
