UPDATE dead_letter_hooks SET id = gen_random_uuid() WHERE id IS NULL;
