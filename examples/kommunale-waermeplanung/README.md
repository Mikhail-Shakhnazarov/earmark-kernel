# Kommunale Wärmeplanung — Wissensbriefing

Diese Demo zeigt, wie aus heterogenen Wissensnotizen eine belastbare Briefingkarte entsteht, ohne dass Kontext als unstrukturierte Chat-Historie weitergereicht wird.

## Das Problem

In Organisationen der Energie- und Wärmeplanung entstehen laufend Notizen, Workshopprotokolle, Rückmeldungen und Berichte. Das Material ist vorhanden, aber die darin enthaltenen Aussagen werden oft nicht als wiederverwendbare Wissenseinheiten gesichert. Reine KI-Zusammenfassungen ohne Zwischenstufe erschweren Nachvollziehbarkeit und Qualitätsprüfung.

## Was diese Demo zeigt

Die Pipeline arbeitet in zwei Stufen: `quellnotiz -> befund -> briefingkarte`. Zuerst werden atomare Befunde mit Herkunft und Unsicherheitsmarkierung extrahiert. Danach wird ausschließlich auf Basis dieser Befunde eine strukturierte Briefingkarte erstellt. Jede Stufe sieht nur die für sie deklarierte Arbeitsoberfläche (bounded context).

## Das Quellkorpus

Das Quellkorpus umfasst fünf Notizen mit bewusst unterschiedlicher Qualität: ein Workshopprotokoll, ein datengestützter Fortschrittsbericht, eine interne Pilotnotiz, eine Stakeholder-Rückmeldung und eine unklare Altnotiz. Die Qualitätsvariation ist absichtlich, um robuste Befundextraktion und sauberen Umgang mit Unsicherheit zu demonstrieren.

## Voraussetzungen (Prerequisites)

Diese Demo setzt voraus, dass das Earmark-CLI gebaut und als `em` verfügbar ist. Vom Repository-Root aus:

```bash
cargo build -p earmark-cli
alias em="$(pwd)/target/debug/earmark-cli"
export REPO_ROOT="$(pwd)"
```

## Demo ausführen

Diese Demo läuft gegen ein externes Workspace außerhalb des Repositorys. So kann Earmark seinen Store sauber verwalten, ohne ein eingebettetes Git-Metadatenverzeichnis im Beispielordner zu benötigen.

```bash
export WORKSPACE=/tmp/earmark-waermeplanung-demo
rm -rf "$WORKSPACE"

# Workspace initialisieren
em --root "$WORKSPACE" init

em --root "$WORKSPACE" declare validate --kind system "$REPO_ROOT/examples/kommunale-waermeplanung/declarations/systems/system.yaml"
em --root "$WORKSPACE" system register "$REPO_ROOT/examples/kommunale-waermeplanung/declarations/systems/system.yaml"
em --root "$WORKSPACE" system activate sys_waermeplanung_briefing

em --root "$WORKSPACE" deposit --class quellnotiz --title "Workshopprotokoll Wärmeplanung" --payload-file "$REPO_ROOT/examples/kommunale-waermeplanung/seed/notiz_1_workshop.md"
em --root "$WORKSPACE" deposit --class quellnotiz --title "Fortschrittsbericht Auszug" --payload-file "$REPO_ROOT/examples/kommunale-waermeplanung/seed/notiz_2_fortschrittsbericht.md"
em --root "$WORKSPACE" deposit --class quellnotiz --title "Pilotprojekt Datenintegration" --payload-file "$REPO_ROOT/examples/kommunale-waermeplanung/seed/notiz_3_pilotprojekt.md"
em --root "$WORKSPACE" deposit --class quellnotiz --title "Stakeholder-Rückmeldung" --payload-file "$REPO_ROOT/examples/kommunale-waermeplanung/seed/notiz_4_stakeholder.md"
em --root "$WORKSPACE" deposit --class quellnotiz --title "Unklare Altnotiz Fernwärme" --payload-file "$REPO_ROOT/examples/kommunale-waermeplanung/seed/notiz_5_unklare_altnotiz.md"

em --root "$WORKSPACE" query --class quellnotiz

em --root "$WORKSPACE" workflow run waermeplanung_briefing --system-id sys_waermeplanung_briefing --with <ID_1> --with <ID_2> --with <ID_3> --with <ID_4> --with <ID_5>

em --root "$WORKSPACE" query --class befund
em --root "$WORKSPACE" query --class briefingkarte
em --root "$WORKSPACE" run explain latest
```

## Ergebnisse prüfen

Prüfen Sie die erzeugte `briefingkarte` und vergleichen Sie sie mit `expected-output/briefingkarte.md`. Achten Sie besonders darauf, dass die Synthese nur Befunde verwendet und unsicheres Material sichtbar ausweist, statt es stillschweigend einzubauen.

## Was diese Demo demonstriert

- Begrenzter Kontext: Jede Stufe sieht nur deklarierte Eingaben.
- Persistente Befunde: Extrahierte Aussagen bleiben als Objekte im Store erhalten.
- Gesteuerter Handoff: Die Briefing-Stufe arbeitet ausschließlich auf Befunden.
- Sichtbare Unsicherheit: Schwaches Material wird markiert statt versteckt.
- Nachvollziehbare Herkunft: Befunde bleiben auf Quellnotizen rückführbar.

## Über Earmark

Earmark ist ein deklarationsorientiertes Laufzeitsystem für gesteuerte KI-Ausführung und dauerhafte Wissensobjekte. Die Demo zeigt, wie aus verteilten Notizen eine prüfbare, weiterverwendbare Wissensgrundlage für Entscheidungen entsteht.
