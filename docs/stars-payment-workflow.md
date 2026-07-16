# Stars balance и invoice payment

F018 оставляет весь payment/gift/Passport surface в generated raw registry и добавляет
минимальный путь, не принимающий card, provider или identity secrets:

```text
star_balance
plan_star_invoice_payment
apply_star_invoice_payment
```

`star_balance` сам получает current user из TDLib и возвращает fresh `starAmount` без
transaction IDs/details. Plan принимает invoice name, заново читает payment form и ledger,
разрешает только `paymentFormTypeStars|paymentFormTypeStarSubscription`, проверяет баланс и
возвращает seller, amount и hash exact `sendPaymentForm`. Invoice name связан хешем, но не
отражается в plan output.

Apply повторно строит plan из fresh form; старый approval не совпадёт при смене form ID.
External one-shot approval обязателен. Core всегда передаёт `credentials=null`, пустые
order/shipping IDs и нулевой tip — ordinary CLI route не может принять card/order/Passport
data. После dispatch success или timeout выполняется только ledger read. Completion требует
новую transaction с exact seller/amount/type; иначе result `uncertain` и send не повторяется.
Verification URL заменяется outcome `verification_required` и не сериализуется.

Synthetic SC001 проверяет свежий balance. SC002 теряет send response и подтверждает ровно
один dispatch с последующей ledger reconciliation. Live spending и approved provider
остаются P10/Q001 и без разрешения владельца не выполняются.
