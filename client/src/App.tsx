import React, { useEffect, useState } from "react";
import "./App.css";

interface Account {
  id: string;
  provider_id: string;
  display_name: string;
}

async function fetchAccounts(): Promise<Account[]> {
  const res = await fetch("/api/accounts");
  return await res.json();
}

function App() {
  const [accounts, setAccounts] = useState<Account[] | null>(null);

  useEffect(() => {
    fetchAccounts().then(setAccounts);
  }, []);

  if (accounts === null) {
    return <p>Loading...</p>;
  }

  if (accounts.length === 0) {
    return <a href="/connect">Connect</a>;
  }

  return (
    <div className="container">
      <h2>Accounts</h2>
      <ul>
        {accounts.map((a) => (
          <li key={a.id}>{a.display_name}</li>
        ))}
      </ul>
    </div>
  );
}

export default App;
