import { linkClass } from "./ui";

export function StatusFooter() {
  return (
    <footer
      data-testid="status-footer"
      className="mx-auto mt-8 flex w-full max-w-5xl flex-wrap items-center justify-between gap-2 border-t border-border-soft py-4 text-small text-meta"
    >
      <p>Support console — metadata only. Every read is audited by the support API.</p>
      <a data-testid="status-footer-link" href="/status" className={linkClass}>
        Public status page
      </a>
    </footer>
  );
}
