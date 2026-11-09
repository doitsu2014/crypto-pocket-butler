import { cardClass, linkClass, overlineClass, sectionTitleClass } from "./ui";

const ARTICLES = [
  {
    id: "connect-an-exchange",
    title: "Connect an exchange",
    summary: "Use a read-only key; withdrawals and trading permissions are never requested.",
  },
  {
    id: "where-is-my-portfolio",
    title: "Where is my portfolio",
    summary: "Portfolio totals appear after the first successful account sync.",
  },
  {
    id: "account-sync-status",
    title: "Account sync status",
    summary: "The account list shows the last synced time and the current data state.",
  },
];

/** BR4.1: the console reuses u2-web-app's help content read-only and links to it. */
export function HelpPanel() {
  return (
    <section data-testid="help-panel" className={cardClass}>
      <p className={overlineClass}>Shared with the customer app</p>
      <h2 className={`${sectionTitleClass} mt-1`}>Help</h2>
      <p className="mt-2 text-small text-ink-2">
        These articles are read-only here. Customers see the same content in the help centre.
      </p>

      <ul className="mt-4 flex flex-col gap-3">
        {ARTICLES.map((article) => (
          <li key={article.id} className="rounded-lg border border-border-soft bg-surface-warm p-3">
            <h3 className="text-small font-medium text-ink">{article.title}</h3>
            <p className="mt-1 text-small text-ink-2">{article.summary}</p>
          </li>
        ))}
      </ul>

      <a
        data-testid="help-panel-contact"
        href="https://cryptopocketbutler.example/help"
        className={`${linkClass} mt-4 inline-block`}
      >
        Open the customer help centre and contact channel
      </a>
    </section>
  );
}
