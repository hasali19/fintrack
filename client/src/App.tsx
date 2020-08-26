import React, { useEffect, useState } from "react";
import {
  AppBar,
  Toolbar,
  Typography,
  Container,
  makeStyles,
  Card,
  CardContent,
  Grid,
  TableContainer,
  Table,
  TableRow,
  TableBody,
  TableCell,
  TableHead,
  Paper,
} from "@material-ui/core";

interface Account {
  id: string;
  provider_id: string;
  display_name: string;
}

interface Balance {
  currency: string;
  available: number;
  current: number;
  overdraft: number;
  update_timestamp: string;
}

interface Transaction {
  id: string;
  description: string;
  amount: number;
  timestamp: string;
}

async function fetchAccounts(): Promise<Account[]> {
  const res = await fetch("/api/accounts");
  return await res.json();
}

async function fetchBalance(account: string): Promise<Balance> {
  const res = await fetch("/api/accounts/" + account + "/balance");
  return await res.json();
}

async function fetchTransactions(account: string): Promise<Transaction[]> {
  const res = await fetch("/api/accounts/" + account + "/transactions");
  return await res.json();
}

const useStyles = makeStyles(({ spacing }) => ({
  spacer: {
    width: "100%",
    height: spacing(2),
  },
  balanceMain: {
    fontSize: 42,
    textAlign: "center",
    color: "greenyellow",
  },
  balanceSecondary: {
    fontSize: 20,
    textAlign: "center",
    color: "lightgrey",
  },
}));

function formatMoney(value: number) {
  return value.toFixed(2).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

function App() {
  const classes = useStyles();
  const [accounts, setAccounts] = useState<Account[] | null>(null);

  const [activeAccount, setActiveAccount] = useState<Account | null>(null);
  const [balance, setBalance] = useState<Balance | null>(null);
  const [transactions, setTransactions] = useState<Transaction[] | null>(null);

  useEffect(() => {
    fetchAccounts().then(setAccounts);
  }, []);

  useEffect(() => {
    if (accounts && accounts.length > 0) {
      setActiveAccount(accounts[0]);
    }
  }, [accounts]);

  useEffect(() => {
    if (activeAccount) {
      fetchBalance(activeAccount.id).then(setBalance);
      fetchTransactions(activeAccount.id).then(setTransactions);
    }
  }, [activeAccount]);

  if (accounts === null) {
    return <p>Loading...</p>;
  }

  if (accounts.length === 0) {
    return <a href="/connect">Connect</a>;
  }

  if (balance === null) {
    return <p>Loading...</p>;
  }

  return (
    <div>
      <AppBar>
        <Toolbar>
          <Typography variant="h6">FinTrack</Typography>
        </Toolbar>
      </AppBar>
      <Toolbar />
      <div className={classes.spacer} />
      <Container className={classes.spacer}>
        <Typography variant="h2">Overview</Typography>
        <div className={classes.spacer} />
        <div className={classes.spacer} />
        <Grid container spacing={2}>
          <Grid item xs={12} sm={6} md={4}>
            <Card style={{ width: "100%", height: "100%" }}>
              <CardContent>
                <Typography variant="h5">
                  {activeAccount?.display_name}
                </Typography>
                {balance && (
                  <>
                    <Typography className={classes.balanceMain}>
                      £{formatMoney(balance.current)}
                    </Typography>
                    <Typography className={classes.balanceSecondary}>
                      £{formatMoney(balance.available - balance.overdraft)}{" "}
                      (inc. pending)
                    </Typography>
                  </>
                )}
              </CardContent>
            </Card>
          </Grid>
        </Grid>
        <div className={classes.spacer} />
        <div className={classes.spacer} />
        <Typography variant="h3">Transactions</Typography>
        <div className={classes.spacer} />
        {transactions && (
          <TableContainer component={Paper}>
            <Table>
              <TableHead>
                <TableRow>
                  <TableCell>Description</TableCell>
                  <TableCell>Amount</TableCell>
                  <TableCell>Date</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {transactions.map((t) => (
                  <TableRow key={t.id}>
                    <TableCell>{t.description}</TableCell>
                    <TableCell
                      style={{
                        color: t.amount < 0 ? "#ff2c2c" : "#b8ff4d",
                      }}
                    >
                      £{formatMoney(Math.abs(t.amount))}
                    </TableCell>
                    <TableCell>
                      {new Date(t.timestamp).toLocaleDateString()}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TableContainer>
        )}
        <div className={classes.spacer} />
        <div className={classes.spacer} />
      </Container>
    </div>
  );
}

export default App;
