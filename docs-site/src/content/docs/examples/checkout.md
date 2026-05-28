---
title: E-commerce checkout flow
description: Record and replay a multi-step checkout test in an Android shopping app.
---

## 1. Create the `.qaly.test` file

```yaml
# shop.qaly.test
app: com.example.shop
clean_state: app_data

- goal: search for a product and add to cart
- goal: complete checkout with credit card
- goal: verify order confirmation screen
```

## 2. Record

Ask your agent:

```
Record all tests in shop.qaly.test.
```

## 3. Replay

```bash
qaly-test shop.qaly.test
```

```
✓ search for a product and add to cart (0.8s)
✓ complete checkout with credit card (1.4s)
✓ verify order confirmation screen (0.6s)
All 3 tests passed in 2.8s.
```

## 4. Run in parallel

```bash
# 3 workers = all 3 tests simultaneously
qaly-test shop.qaly.test --workers 3
# ~1.5s instead of ~2.8s
```
