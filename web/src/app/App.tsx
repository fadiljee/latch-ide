import { AppShell } from "./AppShell";
import { AppProviders } from "./providers";

export function App() {
  return (
    <AppProviders>
      <AppShell />
    </AppProviders>
  );
}

export default App;
