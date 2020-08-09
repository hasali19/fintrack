import React, { useEffect, useState } from "react";

interface SessionState {
  authenticated: boolean;
}

async function fetchSession(): Promise<SessionState> {
  const res = await fetch("/api/session");
  return await res.json();
}

function App() {
  const [session, setSession] = useState<SessionState | null>(null);

  useEffect(() => {
    fetchSession().then(setSession);
  }, []);

  if (session === null) {
    return <p>Loading...</p>;
  }

  if (session.authenticated) {
    return <p>Authenticated!</p>;
  }

  return <a href="/connect">Connect</a>;
}

export default App;
