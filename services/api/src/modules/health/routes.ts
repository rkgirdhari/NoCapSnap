import { Router } from "express";
import { COMPANY } from "@nocapsnap/shared";

export const healthRouter = Router();

healthRouter.get("/", (_req, res) => {
  res.json({
    ok: true,
    product: COMPANY.productName,
    company: COMPANY.legalName,
    founder: COMPANY.founder,
  });
});
