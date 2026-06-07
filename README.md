# Test locally
## seed the data in postgres container
```
docker-compose exec -T postgres psql -U pgmorph -d pgmorph < sql/seed.sql
```