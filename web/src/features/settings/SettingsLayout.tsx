import { Outlet } from "react-router-dom";
import { SettingsNav } from "./SettingsNav";

/**
 * The settings shell: navigation beside the active panel.
 *
 * The shell itself reads nothing and deliberately renders no `QueryBoundary`:
 * its error branch renders `ErrorPanel`, which renders `ProblemDetails.detail`,
 * and a panel's failure must not blank the navigation anyway. Each panel owns
 * its own loading and error states.
 */
export default function SettingsLayout() {
  return (
    <div className="grid gap-8 md:grid-cols-[14rem_minmax(0,1fr)]">
      <SettingsNav />
      <div className="min-w-0">
        <Outlet />
      </div>
    </div>
  );
}
