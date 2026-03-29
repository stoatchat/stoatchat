try {
    rs.status();
} catch (e) {
    rs.initiate({ _id: "rs0", members: [{ _id: 0, host: "localhost:27017" }] });
}
