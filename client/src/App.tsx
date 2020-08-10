import React, { useEffect, useState } from "react";
import "./App.css";

interface Provider {
  id: string;
  name: string;
  logo: string;
}

async function fetchProviders(): Promise<Provider[]> {
  const res = await fetch("/api/providers");
  return await res.json();
}

interface Account {
  account_id: string;
  account_type: string;
  account_number: {
    iban: string | null;
    number: string | null;
    sort_code: string | null;
  };
  currency: string;
  display_name: string;
  provider: {
    provider_id: string;
    display_name: string;
    logo_uri: string;
  };
  description: string;
}

async function fetchAccounts(provider: Provider): Promise<Account[]> {
  const res = await fetch(
    "/api/accounts?provider=" + encodeURIComponent(provider.id)
  );
  return await res.json();
}

type AccountMap = Record<string, Account[] | undefined>;

function App() {
  const [providers, setProviders] = useState<Provider[] | null>(null);
  const [accounts, setAccounts] = useState<Record<
    string,
    Account[] | undefined
  > | null>(null);

  useEffect(() => {
    fetchProviders().then(setProviders);
  }, []);

  useEffect(() => {
    if (providers) {
      providers.map((p) =>
        fetchAccounts(p).then((res) =>
          setAccounts((accs) => ({ ...accs, [p.id]: res }))
        )
      );
    }
  }, [providers]);

  if (providers === null) {
    return <p>Loading...</p>;
  }

  if (providers.length === 0) {
    return <a href="/connect">Connect</a>;
  }

  return (
    <div className="container">
      <h2>Connected providers</h2>
      <div className="providers">
        {providers.map((p) => (
          <div key={p.id}>
            <h3>{p.name}</h3>
            <img src={p.logo} alt={p.name} className="provider-logo" />
            <h4>Accounts</h4>
            <ul>
              {accounts &&
                accounts[p.id]?.map((a) => (
                  <li key={a.account_id}>{a.display_name}</li>
                ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;
