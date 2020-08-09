import React, { useEffect, useState } from "react";
import "./App.css";

interface SessionState {
  authenticated: boolean;
}

async function fetchSession(): Promise<SessionState> {
  const res = await fetch("/api/session");
  return await res.json();
}

interface Provider {
  id: string;
  name: string;
  logo: string;
}

async function fetchProviders(): Promise<Provider[]> {
  const res = await fetch("/api/providers");
  return await res.json();
}

function App() {
  const [session, setSession] = useState<SessionState | null>(null);
  const [providers, setProviders] = useState<Provider[]>([]);

  useEffect(() => {
    fetchSession().then(setSession);
  }, []);

  useEffect(() => {
    if (session?.authenticated) {
      fetchProviders().then(setProviders);
    }
  }, [session]);

  if (session === null) {
    return <p>Loading...</p>;
  }

  if (!session.authenticated) {
    return <a href="/connect">Connect</a>;
  }

  return (
    <div className="container">
      <h2>Authenticated!</h2>
      {providers.length === 0 ? (
        <p>
          No connected providers (<a href="/connect">connect</a>)
        </p>
      ) : (
        <>
          <h3>Connected providers</h3>
          <div className="providers">
            {providers.map((p) => (
              <div className="provider">
                <img src={p.logo} alt={p.name} className="provider-logo" />
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

export default App;
